//! Bounded, in-memory connection metadata. Never retains payloads or credentials.
use crate::BoxStream;
use serde::Serialize;
use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
const HISTORY: usize = 256;
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
#[derive(Clone, Serialize, Default)]
pub struct Diagnostic {
    pub status: String,
    pub mode: String,
    pub route: Option<String>,
    pub exit_ip: Option<String>,
    pub server_ip: String,
    pub matches_server: Option<bool>,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
    pub checked_at: u64,
}
#[derive(Serialize)]
pub struct Snapshot {
    pub schema: u8,
    pub session: u64,
    pub mode: String,
    pub captured_only: bool,
    pub active: usize,
    pub upload: u64,
    pub download: u64,
    pub history_limit: usize,
    pub connections: Vec<Connection>,
    pub diagnostic: Diagnostic,
}
#[derive(Serialize)]
pub struct Connection {
    pub id: u64,
    pub host: String,
    pub source: String,
    pub protocol: String,
    pub port: u16,
    pub address: Option<String>,
    pub route: String,
    pub rule_line: Option<usize>,
    pub rule: Option<String>,
    pub status: String,
    pub reason: Option<String>,
    pub started_at: u64,
    pub duration_ms: u64,
    pub connect_ms: Option<u64>,
    pub upload: u64,
    pub download: u64,
}
struct Metadata {
    address: Option<String>,
    route: String,
    rule_line: Option<usize>,
    rule: Option<String>,
    status: String,
    reason: Option<String>,
    connect_ms: Option<u64>,
}
pub struct Record {
    id: u64,
    host: String,
    source: String,
    protocol: String,
    port: u16,
    started: Instant,
    started_at: u64,
    ended_ms: AtomicU64,
    closed: AtomicBool,
    upload: AtomicU64,
    download: AtomicU64,
    meta: Mutex<Metadata>,
}
pub struct Monitor {
    session: u64,
    mode: String,
    next: AtomicU64,
    upload: AtomicU64,
    download: AtomicU64,
    records: Mutex<VecDeque<Arc<Record>>>,
    diagnostic: Mutex<Diagnostic>,
}
impl Monitor {
    pub fn new(mode: &str, server_ip: String) -> Arc<Self> {
        Arc::new(Self {
            session: rand::random::<u64>() & ((1u64 << 53) - 1),
            mode: mode.into(),
            next: AtomicU64::new(1),
            upload: AtomicU64::new(0),
            download: AtomicU64::new(0),
            records: Mutex::new(VecDeque::new()),
            diagnostic: Mutex::new(Diagnostic {
                status: "idle".into(),
                mode: mode.into(),
                server_ip,
                ..Default::default()
            }),
        })
    }
    pub fn begin(
        self: &Arc<Self>,
        host: &str,
        port: u16,
        protocol: &str,
        source: &str,
    ) -> FlowGuard {
        let record = Arc::new(Record {
            id: self.next.fetch_add(1, Ordering::Relaxed),
            host: host.chars().take(253).collect(),
            source: source.chars().take(80).collect(),
            protocol: protocol.into(),
            port,
            started: Instant::now(),
            started_at: now(),
            ended_ms: AtomicU64::new(0),
            closed: AtomicBool::new(false),
            upload: AtomicU64::new(0),
            download: AtomicU64::new(0),
            meta: Mutex::new(Metadata {
                address: None,
                route: "pending".into(),
                rule_line: None,
                rule: None,
                status: "connecting".into(),
                reason: None,
                connect_ms: None,
            }),
        });
        let mut records = self.records.lock().unwrap();
        if records.len() >= HISTORY {
            if let Some(index) = records
                .iter()
                .position(|r| r.closed.load(Ordering::Acquire))
            {
                records.remove(index);
            } else {
                records.pop_front();
            }
        }
        records.push_back(record.clone());
        FlowGuard {
            record,
            monitor: self.clone(),
        }
    }
    pub fn clear_history(&self) {
        self.records
            .lock()
            .unwrap()
            .retain(|r| !r.closed.load(Ordering::Acquire));
    }
    pub fn snapshot(&self) -> Snapshot {
        let records = self
            .records
            .lock()
            .unwrap()
            .iter()
            .rev()
            .cloned()
            .collect::<Vec<_>>();
        let mut active = 0;
        let connections = records
            .into_iter()
            .map(|r| {
                let meta = r.meta.lock().unwrap();
                let closed = r.closed.load(Ordering::Acquire);
                if !closed {
                    active += 1;
                }
                Connection {
                    id: r.id,
                    host: r.host.clone(),
                    source: r.source.clone(),
                    protocol: r.protocol.clone(),
                    port: r.port,
                    address: meta.address.clone(),
                    route: meta.route.clone(),
                    rule_line: meta.rule_line,
                    rule: meta.rule.clone(),
                    status: meta.status.clone(),
                    reason: meta.reason.clone(),
                    started_at: r.started_at,
                    duration_ms: if closed {
                        r.ended_ms.load(Ordering::Relaxed)
                    } else {
                        r.started.elapsed().as_millis() as u64
                    },
                    connect_ms: meta.connect_ms,
                    upload: r.upload.load(Ordering::Relaxed),
                    download: r.download.load(Ordering::Relaxed),
                }
            })
            .collect();
        Snapshot {
            schema: 1,
            session: self.session,
            mode: self.mode.clone(),
            captured_only: true,
            active,
            upload: self.upload.load(Ordering::Relaxed),
            download: self.download.load(Ordering::Relaxed),
            history_limit: HISTORY,
            connections,
            diagnostic: self.diagnostic.lock().unwrap().clone(),
        }
    }
    pub fn begin_diagnostic(&self) -> bool {
        let mut d = self.diagnostic.lock().unwrap();
        if d.status == "running" {
            return false;
        }
        *d = Diagnostic {
            status: "running".into(),
            mode: self.mode.clone(),
            server_ip: d.server_ip.clone(),
            checked_at: now(),
            ..Default::default()
        };
        true
    }
    pub fn diagnostic_route(&self, route: &str) {
        if ["direct", "proxy", "block"].contains(&route) {
            self.diagnostic.lock().unwrap().route = Some(route.into());
        }
    }
    pub fn finish_diagnostic(&self, result: std::io::Result<(String, String)>, elapsed: u64) {
        let mut d = self.diagnostic.lock().unwrap();
        d.latency_ms = Some(elapsed);
        d.checked_at = now();
        match result {
            Ok((ip, route)) => {
                d.matches_server = Some(ip == d.server_ip);
                d.exit_ip = Some(ip);
                d.route = Some(route);
                d.status = "success".into();
            }
            Err(_) => {
                d.status = "failed".into();
                d.error = Some("probe_failed".into());
            }
        }
    }
    pub fn stopped(&self) {
        let mut d = self.diagnostic.lock().unwrap();
        if d.status == "running" {
            d.status = "failed".into();
            d.error = Some("disconnected".into());
        }
    }
}
pub struct FlowGuard {
    record: Arc<Record>,
    monitor: Arc<Monitor>,
}
impl FlowGuard {
    pub fn route(
        &self,
        address: std::net::SocketAddr,
        route: &str,
        line: Option<usize>,
        rule: Option<&str>,
    ) {
        let mut m = self.record.meta.lock().unwrap();
        m.address = Some(address.to_string());
        m.route = route.into();
        m.rule_line = line;
        m.rule = rule.map(|s| s.chars().take(300).collect());
    }
    pub fn route_name(&self) -> String {
        self.record.meta.lock().unwrap().route.clone()
    }
    pub fn connected(&self) {
        let mut m = self.record.meta.lock().unwrap();
        m.status = "active".into();
        m.connect_ms = Some(self.record.started.elapsed().as_millis() as u64);
    }
    pub fn failed(&self, reason: &str) {
        let mut m = self.record.meta.lock().unwrap();
        if m.status == "blocked" || m.status == "failed" {
            return;
        }
        m.status = if reason == "blocked" {
            "blocked"
        } else {
            "failed"
        }
        .into();
        m.reason = Some(reason.into());
    }
    pub fn upload(&self, n: usize) {
        self.record.upload.fetch_add(n as u64, Ordering::Relaxed);
        self.monitor.upload.fetch_add(n as u64, Ordering::Relaxed);
    }
    pub fn download(&self, n: usize) {
        self.record.download.fetch_add(n as u64, Ordering::Relaxed);
        self.monitor.download.fetch_add(n as u64, Ordering::Relaxed);
    }
    pub fn stream(self, inner: BoxStream) -> BoxStream {
        Box::new(TrackedStream { inner, flow: self })
    }
}
impl Drop for FlowGuard {
    fn drop(&mut self) {
        let mut m = self.record.meta.lock().unwrap();
        if m.status == "active" || m.status == "connecting" {
            m.status = "closed".into();
        }
        self.record.ended_ms.store(
            self.record.started.elapsed().as_millis() as u64,
            Ordering::Relaxed,
        );
        self.record.closed.store(true, Ordering::Release);
    }
}
struct TrackedStream {
    inner: BoxStream,
    flow: FlowGuard,
}
impl AsyncRead for TrackedStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);
        if let Poll::Ready(ref r) = result {
            if r.is_ok() {
                self.flow.download(buf.filled().len() - before);
            } else {
                self.flow.failed("io_error");
            }
        }
        result
    }
}
impl AsyncWrite for TrackedStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let result = Pin::new(&mut self.inner).poll_write(cx, data);
        if let Poll::Ready(ref r) = result {
            match r {
                Ok(n) => self.flow.upload(*n),
                Err(_) => self.flow.failed("io_error"),
            }
        }
        result
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn byte_counts_lifecycle_and_bounds() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let monitor = Monitor::new("auto", "203.0.113.1".into());
        let flow = monitor.begin("example.com", 443, "tcp", "capture");
        flow.route(
            "1.1.1.1:443".parse().unwrap(),
            "proxy",
            Some(1),
            Some("final proxy:default"),
        );
        flow.connected();
        let (client, mut peer) = tokio::io::duplex(64);
        let mut stream = flow.stream(Box::new(client));
        stream.write_all(b"hello").await.unwrap();
        let mut request = [0; 5];
        peer.read_exact(&mut request).await.unwrap();
        peer.write_all(b"world!").await.unwrap();
        let mut reply = [0; 6];
        stream.read_exact(&mut reply).await.unwrap();
        let live = monitor.snapshot();
        assert_eq!((live.upload, live.download, live.active), (5, 6, 1));
        monitor.clear_history();
        assert_eq!(monitor.snapshot().connections.len(), 1);
        drop(stream);
        assert_eq!(monitor.snapshot().connections[0].status, "closed");
        for _ in 0..400 {
            drop(monitor.begin("x.test", 80, "tcp", "proxy"));
        }
        let end = monitor.snapshot();
        assert_eq!(end.connections.len(), HISTORY);
        assert_eq!((end.upload, end.download, end.active), (5, 6, 0));
        monitor.clear_history();
        assert!(monitor.snapshot().connections.is_empty());
    }
    #[test]
    fn failures_and_diagnostic_single_flight() {
        let m = Monitor::new("direct", "1.2.3.4".into());
        let f = m.begin("blocked.test", 443, "tcp", "capture");
        f.failed("blocked");
        drop(f);
        assert_eq!(m.snapshot().connections[0].status, "blocked");
        assert!(m.begin_diagnostic());
        assert!(!m.begin_diagnostic());
        m.finish_diagnostic(Ok(("1.2.3.4".into(), "direct".into())), 12);
        assert_eq!(m.snapshot().diagnostic.route.as_deref(), Some("direct"));
        assert_eq!(m.snapshot().diagnostic.matches_server, Some(true));
        assert!(m.begin_diagnostic());
        m.stopped();
        assert_eq!(m.snapshot().diagnostic.status, "failed");
    }
}

//! Shared TCP/UDP policy engine for desktop TUN, Android VpnService and Apple PacketTunnel.
#[cfg(target_os = "android")]
mod android;
mod cache;
mod diagnostic;
mod dns;
pub mod ffi;
pub mod monitor;
mod proxy;
pub mod socket;
use futures::{SinkExt, StreamExt};
use routeloom_core::{Action, Flow, Network, Policy};
use routeloom_tunnel::{
    config::{Profile, RoutingMode},
    flow::{self, Kind},
    invalid, wire, Result,
};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{mpsc, Mutex, Semaphore},
    task::JoinSet,
};
pub trait Stream: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin> Stream for T {}
pub type BoxStream = Box<dyn Stream>;
pub struct Engine {
    pub input: mpsc::Sender<Vec<u8>>,
    pub output: mpsc::Receiver<Vec<u8>>,
    /// 0 connecting/reconnecting, 1 connected, 2 stopped/error.
    pub state: Arc<AtomicU8>,
    pub monitor: Arc<monitor::Monitor>,
    pub commands: mpsc::Sender<()>,
    task: tokio::task::JoinHandle<()>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
}
impl Engine {
    pub async fn shutdown(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        let _ = (&mut self.task).await;
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        self.task.abort();
    }
}
pub(crate) struct Context {
    profile: Profile,
    policy: Policy,
    protect: socket::Protector,
    names: Mutex<dns::Names>,
    state: Arc<AtomicU8>,
    permits: Arc<Semaphore>,
    monitor: Arc<monitor::Monitor>,
}
impl Context {
    async fn open(
        &self,
        kind: Kind,
        host: &str,
        port: u16,
    ) -> Result<(
        tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
        Vec<IpAddr>,
    )> {
        let tcp = socket::tcp(self.profile.endpoint, &self.protect).await?;
        flow::open(&self.profile, tcp, kind, host, port).await
    }
    async fn destination(
        &self,
        host: &str,
        port: u16,
        network: Network,
        observed: &monitor::FlowGuard,
    ) -> Result<(SocketAddr, Action)> {
        if self.state.load(Ordering::Acquire) != 1 {
            observed.failed("engine_unavailable");
            return Err(invalid("tunnel unavailable"));
        }
        let resolved = async {
            if let Ok(ip) = host.parse::<IpAddr>() {
                Ok(vec![ip])
            } else if self.profile.options.mode == RoutingMode::Direct {
                dns::resolve_direct(host, &self.profile.dns, &self.protect).await
            } else {
                Ok(self.open(Kind::Resolve, host, 0).await?.1)
            }
        }
        .await;
        let ips = resolved.map_err(|e| {
            observed.failed("resolution_failed");
            e
        })?;
        let ip = ips
            .iter()
            .find(|ip| ip.is_ipv4())
            .or(ips.first())
            .copied()
            .ok_or_else(|| invalid("DNS returned no addresses"))?;
        let flow =
            Flow::new(host, Some(ip), port, network).map_err(|_| invalid("invalid destination"))?;
        let (action, line) = match self.profile.options.mode {
            RoutingMode::Auto => {
                let d = self.policy.evaluate(&flow);
                (d.action, Some(d.line))
            }
            RoutingMode::Proxy => (Action::Proxy("default".into()), None),
            RoutingMode::Direct => (Action::Direct, None),
        };
        let route = match &action {
            Action::Direct => "direct",
            Action::Block => "block",
            Action::Proxy(_) => "proxy",
        };
        let address = SocketAddr::new(ip, port);
        observed.route(
            address,
            route,
            line,
            line.and_then(|n| self.profile.options.policy.lines().nth(n.saturating_sub(1))),
        );
        if action == Action::Block {
            observed.failed("blocked");
            return Err(invalid("blocked by policy"));
        }
        Ok((address, action))
    }
    pub(crate) async fn tcp(&self, host: &str, port: u16, source: &str) -> Result<BoxStream> {
        let (stream, _) = self.tcp_with_route(host, port, source).await?;
        Ok(stream)
    }
    async fn tcp_with_route(
        &self,
        host: &str,
        port: u16,
        source: &str,
    ) -> Result<(BoxStream, String)> {
        let observed = self.monitor.begin(host, port, "tcp", source);
        let result = tokio::time::timeout(Duration::from_secs(20), async {
            let (address, action) = self
                .destination(host, port, Network::Tcp, &observed)
                .await?;
            let stream: BoxStream = match action {
                Action::Direct => Box::new(socket::tcp(address, &self.protect).await?),
                Action::Proxy(group) if group == "default" => Box::new(
                    self.open(Kind::Tcp, &address.ip().to_string(), port)
                        .await?
                        .0,
                ),
                _ => return Err(invalid("unsupported proxy group")),
            };
            Ok(stream)
        })
        .await;
        if source == "diagnostic" {
            self.monitor.diagnostic_route(&observed.route_name());
        }
        match result {
            Ok(Ok(stream)) => {
                observed.connected();
                let route = observed.route_name();
                Ok((observed.stream(stream), route))
            }
            Ok(Err(e)) => {
                observed.failed("connect_failed");
                Err(e)
            }
            Err(_) => {
                observed.failed("timeout");
                Err(invalid("connection timeout"))
            }
        }
    }
    async fn wait_offline(&self) {
        while self.state.load(Ordering::Acquire) == 1 {
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
    async fn heartbeat(&self) -> Result<()> {
        let (mut stream, _) = self.open(Kind::Ping, "", 0).await?;
        self.state.store(1, Ordering::Release);
        loop {
            wire::write_frame(&mut stream, wire::PING, &[]).await?;
            let (kind, _) =
                tokio::time::timeout(Duration::from_secs(15), wire::read_frame(&mut stream))
                    .await
                    .map_err(|_| invalid("heartbeat timeout"))??;
            if kind != wire::PONG {
                return Err(invalid("invalid heartbeat"));
            }
            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    }
}
pub fn start(profile: Profile, protect: socket::Protector) -> Result<Engine> {
    start_in(profile, protect, cache::default_directory()?)
}
pub fn start_in(
    profile: Profile,
    protect: socket::Protector,
    directory: std::path::PathBuf,
) -> Result<Engine> {
    profile.validate()?;
    let policy =
        Policy::parse(&profile.options.policy).map_err(|_| invalid("invalid routing policy"))?;
    policy
        .validate_default_group()
        .map_err(|_| invalid("unsupported proxy group"))?;
    let (input, mut packets) = mpsc::channel::<Vec<u8>>(256);
    let (return_tx, output) = mpsc::channel(256);
    let state = Arc::new(AtomicU8::new(0));
    let names = dns::Names::open(&directory, &profile)?;
    let proxy_listener = if profile.options.pac_port == 0 {
        None
    } else {
        let listener = std::net::TcpListener::bind(("127.0.0.1", profile.options.pac_port))?;
        listener.set_nonblocking(true)?;
        Some(tokio::net::TcpListener::from_std(listener)?)
    };
    let monitor = monitor::Monitor::new(
        profile.options.mode.as_str(),
        profile.endpoint.ip().to_string(),
    );
    let (commands, mut requests) = mpsc::channel::<()>(1);
    let context = Arc::new(Context {
        profile,
        policy,
        protect,
        names: Mutex::new(names),
        state: state.clone(),
        permits: Arc::new(Semaphore::new(64)),
        monitor: monitor.clone(),
    });
    let (stack, runner, udp, tcp) = netstack_smoltcp::StackBuilder::default()
        .enable_tcp(true)
        .enable_udp(true)
        .enable_icmp(false)
        .stack_buffer_size(256)
        .tcp_buffer_size(128)
        .udp_buffer_size(128)
        .tcp_recv_buffer_size(32768)
        .tcp_send_buffer_size(32768)
        .mtu(1280)
        .build()?;
    let (stop, mut stopped) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let mut tasks = JoinSet::new();
        if let Some(runner) = runner {
            tasks.spawn(async move {
                let _ = runner.await;
            });
        }
        let c = context.clone();
        tasks.spawn(async move {
            if c.profile.options.mode == RoutingMode::Direct {
                c.state.store(1, Ordering::Release);
                std::future::pending::<()>().await;
            }
            let mut attempt = 0u32;
            loop {
                let began = tokio::time::Instant::now();
                if let Err(error) = c.heartbeat().await {
                    eprintln!("RouteLoom transport unavailable: {:?}", error.kind());
                }
                if began.elapsed() >= Duration::from_secs(60) {
                    attempt = 0;
                }
                c.state.store(0, Ordering::Release);
                if !c.profile.options.auto_reconnect {
                    c.state.store(2, Ordering::Release);
                    break;
                }
                let delay = backoff(attempt, rand::random::<u16>());
                attempt = attempt.saturating_add(1);
                tokio::time::sleep(delay).await;
            }
        });
        let (mut sink, mut stream) = stack.split();
        let c = context.clone();
        tasks.spawn(async move {
            while let Some(packet) = packets.recv().await {
                if packet.len() > 1280 {
                    continue;
                }
                let Ok((src, _)) = routeloom_tunnel::packet::addresses(&packet) else {
                    continue;
                };
                if src != c.profile.ipv4 && src != c.profile.ipv6 {
                    continue;
                }
                if sink.send(packet).await.is_err() {
                    break;
                }
            }
        });
        tasks.spawn(async move {
            while let Some(Ok(packet)) = stream.next().await {
                if return_tx.send(packet).await.is_err() {
                    break;
                }
            }
        });
        let c = context.clone();
        let mut tcp = tcp.unwrap();
        tasks.spawn(async move{
            let mut flows=JoinSet::new();
            loop {
                tokio::select! {
                    item=tcp.next()=>{let Some((mut stream,source,destination))=item else {break};
                        let Ok(permit)=c.permits.clone().try_acquire_owned()else{continue};
                        let c=c.clone();
                        flows.spawn(async move{
                            let _permit=permit;
                            let work=async{
                                if destination.port()==53 {
                                    loop{
                                        let n=stream.read_u16().await? as usize;
                                        if n>4096 {return Err(invalid("DNS message too large"));}
                                        let mut data=vec![0;n];stream.read_exact(&mut data).await?;
                                        let reply=c.names.lock().await.answer(&data)?;
                                        stream.write_u16(reply.len() as u16).await?;stream.write_all(&reply).await?;
                                    }
                                }
                                let host=c.names.lock().await.host(destination.ip())?;
                                let mut remote=c.tcp(&host,destination.port(),&source.to_string()).await?;
                                tokio::select!{r=tokio::io::copy_bidirectional(&mut stream,&mut remote)=>{r?;},_=c.wait_offline()=>{}};Ok::<(),std::io::Error>(())
                            };
                            let _=tokio::time::timeout(Duration::from_secs(86400),work).await;
                        });
                    },
                    _=flows.join_next(),if !flows.is_empty()=>{}
                }
            }
        });
        let c = context.clone();
        tasks.spawn(async move {
            run_udp(c, udp.unwrap()).await;
        });
        if let Some(listener) = proxy_listener {
            let c = context.clone();
            tasks.spawn(async move {
                if proxy::serve(c.clone(), listener).await.is_err() {
                    c.state.store(2, Ordering::Release);
                }
            });
        }
        // Task cancellation drops JoinSet, stopping every socket and flow.
        loop {
            let result = tokio::select! {
                _ = &mut stopped => break,
                Some(()) = requests.recv() => {
                    if context.monitor.begin_diagnostic() {
                        let c=context.clone();
                        tasks.spawn(async move { diagnostic::run(c).await; });
                    }
                    continue;
                },
                result = tasks.join_next() => result
            };
            let Some(result) = result else { break };
            if result.is_err() {
                break;
            }
            // Heartbeat ending is intentional when automatic reconnect is disabled.
            if context.state.load(Ordering::Acquire) == 2 {
                break;
            }
        }
        context.state.store(2, Ordering::Release);
        tasks.shutdown().await;
        context.monitor.stopped();
    });
    Ok(Engine {
        input,
        output,
        state,
        task,
        stop: Some(stop),
        monitor,
        commands,
    })
}
pub fn backoff(attempt: u32, jitter: u16) -> Duration {
    Duration::from_millis((1000u64 << attempt.min(5)).min(30000) + u64::from(jitter % 501))
}
async fn run_udp(c: Arc<Context>, socket: netstack_smoltcp::UdpSocket) {
    let (mut read, mut write) = socket.split();
    let (responses, mut replies) = mpsc::channel(128);
    let mut routes: HashMap<(SocketAddr, SocketAddr), mpsc::Sender<Vec<u8>>> = HashMap::new();
    let mut flows = JoinSet::new();
    loop {
        tokio::select! {
            item=read.next()=>{
                let Some((data,src,dst))=item else{break};
                if dst.port()==53 {
                    if let Ok(reply)=c.names.lock().await.answer(&data){let _=write.send((reply,dst,src)).await;}continue;
                }
                if data.len()>1280 || c.state.load(Ordering::Acquire)!=1 {continue;}
                routes.retain(|_,tx|!tx.is_closed());
                if let Some(tx)=routes.get(&(src,dst)){let _=tx.try_send(data);continue;}
                let Ok(permit)=c.permits.clone().try_acquire_owned()else{continue};
                let (tx,rx)=mpsc::channel(16);let _=tx.try_send(data);routes.insert((src,dst),tx);
                let c=c.clone();let responses=responses.clone();
                flows.spawn(async move{let _permit=permit;tokio::select!{_=udp_flow(c.clone(),src,dst,rx,responses)=>{},_=c.wait_offline()=>{}}});
            },
            reply=replies.recv()=>if let Some(reply)=reply {let _=write.send(reply).await;},
            _=flows.join_next(),if !flows.is_empty()=>{}
        }
    }
}
async fn udp_flow(
    c: Arc<Context>,
    src: SocketAddr,
    dst: SocketAddr,
    mut rx: mpsc::Receiver<Vec<u8>>,
    tx: mpsc::Sender<(Vec<u8>, SocketAddr, SocketAddr)>,
) -> Result<()> {
    let host = c.names.lock().await.host(dst.ip())?;
    let observed = c.monitor.begin(&host, dst.port(), "udp", &src.to_string());
    let work=async {
        let (address,action)=tokio::time::timeout(Duration::from_secs(20),c.destination(&host,dst.port(),Network::Udp,&observed))
            .await.map_err(|_|invalid("UDP setup timeout"))??;
        match action {
            Action::Direct=>{
                let socket=socket::udp(address,&c.protect).await?;
                observed.connected();
                let mut buf=[0;1281];
                loop {
                    tokio::select! {
                        data=rx.recv()=>{let Some(data)=data else{return Ok(())};let n=socket.send(&data).await?;observed.upload(n);},
                        result=tokio::time::timeout(Duration::from_secs(90),socket.recv(&mut buf))=>{
                            let Ok(result)=result else {return Ok(())};
                            let n=result?;if n<=1280 {observed.download(n);let _=tx.try_send((buf[..n].to_vec(),dst,src));}
                        }
                    }
                }
            }
            Action::Proxy(group) if group=="default"=>{
                let (stream,_)=tokio::time::timeout(Duration::from_secs(20),c.open(Kind::Udp,&address.ip().to_string(),address.port()))
                    .await.map_err(|_|invalid("UDP setup timeout"))??;
                observed.connected();
                let (mut read,mut write)=tokio::io::split(stream);
                let upload=async {
                    while let Some(data)=rx.recv().await {wire::write_frame(&mut write,wire::PACKET,&data).await?;observed.upload(data.len());}
                    Ok::<(),std::io::Error>(())
                };
                let download=async {
                    loop {
                        let Ok(result)=tokio::time::timeout(Duration::from_secs(90),wire::read_frame(&mut read)).await else {return Ok(())};
                        let (kind,data)=result?;
                        if kind!=wire::PACKET || data.len()>1280 {return Err(invalid("invalid UDP frame"));}
                        observed.download(data.len());let _=tx.try_send((data,dst,src));
                    }
                };
                tokio::select!{r=upload=>r,r=download=>r}
            }
            _=>Err(invalid("blocked UDP")),
        }
    }.await;
    if work.is_err() {
        observed.failed("io_error");
    }
    work
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retries_are_bounded() {
        assert_eq!(backoff(0, 0), Duration::from_secs(1));
        assert!(backoff(100, 65535) <= Duration::from_millis(30500));
    }

    fn profile(mode: RoutingMode) -> Profile {
        serde_json::from_value(serde_json::json!({
            "endpoint":"127.0.0.1:9","server_name":"localhost","ca_pem":"unused",
            "device":"mode-test","token":"a".repeat(64),"ipv4":"10.77.0.2","ipv6":"fd77::2",
            "routes":["0.0.0.0/0"],"dns":["1.1.1.1"],
            "options":{"mode":mode,"pac_port":0,"policy":"port 443 block\nfinal direct"}
        }))
        .unwrap()
    }
    #[tokio::test]
    async fn mode_override_and_matched_rule_are_explicit() {
        for mode in [RoutingMode::Auto, RoutingMode::Proxy, RoutingMode::Direct] {
            let profile = profile(mode);
            let monitor = monitor::Monitor::new(mode.as_str(), "127.0.0.1".into());
            let c = Context {
                policy: Policy::parse(&profile.options.policy).unwrap(),
                profile,
                protect: Arc::new(|_, _| Ok(())),
                names: Mutex::new(dns::Names::default()),
                state: Arc::new(AtomicU8::new(1)),
                permits: Arc::new(Semaphore::new(64)),
                monitor: monitor.clone(),
            };
            let observed = monitor.begin("1.2.3.4", 443, "tcp", "test");
            let result = c.destination("1.2.3.4", 443, Network::Tcp, &observed).await;
            match mode {
                RoutingMode::Auto => {
                    assert!(result.is_err());
                    assert_eq!(observed.route_name(), "block");
                }
                RoutingMode::Proxy => {
                    assert_eq!(result.unwrap().1, Action::Proxy("default".into()))
                }
                RoutingMode::Direct => assert_eq!(result.unwrap().1, Action::Direct),
            }
            drop(observed);
            let row = &monitor.snapshot().connections[0];
            assert_eq!(
                row.rule_line,
                if mode == RoutingMode::Auto {
                    Some(1)
                } else {
                    None
                }
            );
        }
    }
    #[tokio::test]
    async fn direct_mode_starts_without_a_server_or_valid_server_ca() {
        let directory =
            std::env::temp_dir().join(format!("routeloom-direct-{}", rand::random::<u64>()));
        let mut engine = start_in(
            profile(RoutingMode::Direct),
            Arc::new(|_, _| Ok(())),
            directory.clone(),
        )
        .unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while engine.state.load(Ordering::Acquire) != 1 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(engine.monitor.snapshot().mode, "direct");
        engine.stop.take().unwrap().send(()).unwrap();
        let _ = (&mut engine.task).await;
        assert_eq!(engine.state.load(Ordering::Acquire), 2);
        drop(engine);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

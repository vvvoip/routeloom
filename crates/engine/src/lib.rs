//! Shared TCP/UDP policy engine for desktop TUN, Android VpnService and Apple PacketTunnel.
#[cfg(target_os = "android")]
mod android;
mod cache;
mod dns;
pub mod ffi;
mod proxy;
pub mod socket;
use futures::{SinkExt, StreamExt};
use routeloom_core::{Action, Flow, Network, Policy};
use routeloom_tunnel::{
    config::Profile,
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
    task: tokio::task::JoinHandle<()>,
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
    ) -> Result<(SocketAddr, Action)> {
        if self.state.load(Ordering::Acquire) != 1 {
            return Err(invalid("tunnel unavailable"));
        }
        let ips = if let Ok(ip) = host.parse::<IpAddr>() {
            vec![ip]
        } else {
            self.open(Kind::Resolve, host, 0).await?.1
        };
        // Prefer IPv4 when both are present; the full policy is evaluated against the chosen IP.
        let ip = ips
            .iter()
            .find(|ip| ip.is_ipv4())
            .or(ips.first())
            .copied()
            .ok_or_else(|| invalid("DNS returned no addresses"))?;
        let flow =
            Flow::new(host, Some(ip), port, network).map_err(|_| invalid("invalid destination"))?;
        let action = self.policy.evaluate(&flow).action;
        if action == Action::Block {
            return Err(invalid("blocked by policy"));
        }
        Ok((SocketAddr::new(ip, port), action))
    }
    pub(crate) async fn tcp(&self, host: &str, port: u16) -> Result<BoxStream> {
        let (address, action) = self.destination(host, port, Network::Tcp).await?;
        match action {
            Action::Direct => Ok(Box::new(socket::tcp(address, &self.protect).await?)),
            Action::Proxy(group) if group == "default" => Ok(Box::new(
                self.open(Kind::Tcp, &address.ip().to_string(), port)
                    .await?
                    .0,
            )),
            _ => Err(invalid("unsupported proxy group")),
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
    let context = Arc::new(Context {
        profile,
        policy,
        protect,
        names: Mutex::new(names),
        state: state.clone(),
        permits: Arc::new(Semaphore::new(64)),
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
    let task = tokio::spawn(async move {
        let mut tasks = JoinSet::new();
        if let Some(runner) = runner {
            tasks.spawn(async move {
                let _ = runner.await;
            });
        }
        let c = context.clone();
        tasks.spawn(async move {
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
                    item=tcp.next()=>{let Some((mut stream,_,destination))=item else {break};
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
                                let mut remote=c.tcp(&host,destination.port()).await?;
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
        while let Some(result) = tasks.join_next().await {
            if result.is_err() {
                break;
            }
            // Heartbeat ending is intentional when automatic reconnect is disabled.
            if context.state.load(Ordering::Acquire) == 2 {
                break;
            }
        }
        context.state.store(2, Ordering::Release);
    });
    Ok(Engine {
        input,
        output,
        state,
        task,
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
    let (address, action) = c.destination(&host, dst.port(), Network::Udp).await?;
    match action {
        Action::Direct => {
            let socket = socket::udp(address, &c.protect).await?;
            let mut buf = [0; 1281];
            loop {
                tokio::select! {
                    data=rx.recv()=>{let Some(data)=data else{return Ok(())};socket.send(&data).await?;},
                    result=tokio::time::timeout(Duration::from_secs(90),socket.recv(&mut buf))=>{let n=result.map_err(|_|invalid("UDP idle timeout"))??;if n<=1280{let _=tx.try_send((buf[..n].to_vec(),dst,src));}}
                }
            }
        }
        Action::Proxy(group) if group == "default" => {
            let (stream, _) = c
                .open(Kind::Udp, &address.ip().to_string(), address.port())
                .await?;
            let (mut read, mut write) = tokio::io::split(stream);
            let upload = async {
                while let Some(data) = rx.recv().await {
                    wire::write_frame(&mut write, wire::PACKET, &data).await?;
                }
                Ok::<(), std::io::Error>(())
            };
            let download = async {
                loop {
                    let (kind, data) =
                        tokio::time::timeout(Duration::from_secs(90), wire::read_frame(&mut read))
                            .await
                            .map_err(|_| invalid("UDP idle timeout"))??;
                    if kind != wire::PACKET {
                        return Err(invalid("invalid UDP frame"));
                    }
                    let _ = tx.try_send((data, dst, src));
                }
            };
            tokio::select! {r=upload=>r,r=download=>r}
        }
        _ => Err(invalid("blocked UDP")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retries_are_bounded() {
        assert_eq!(backoff(0, 0), Duration::from_secs(1));
        assert!(backoff(100, 65535) <= Duration::from_millis(30500));
    }
}

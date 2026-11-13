use ipnet::IpNet;
use routeloom_tunnel::{
    config::{validate_identity, validate_lease, Device},
    invalid, packet, tls, wire, Result,
};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::TcpListener,
    sync::{mpsc, Mutex, Semaphore},
};
use tokio_rustls::TlsAcceptor;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub listen: SocketAddr,
    pub certificate: String,
    pub private_key: String,
    pub devices: Vec<Device>,
    #[serde(default)]
    pub allow_private: Vec<IpNet>,
}
type Sessions = Arc<Mutex<HashMap<IpAddr, mpsc::Sender<Vec<u8>>>>>;
impl Config {
    fn validate(&self) -> Result<()> {
        if self.listen.port() == 0 || self.devices.is_empty() || self.devices.len() > 253 {
            return Err(invalid("invalid server configuration"));
        }
        let mut ids = HashSet::new();
        let mut ips = HashSet::new();
        for d in &self.devices {
            validate_identity(&d.id, &d.token_sha256)?;
            validate_lease(d.ipv4, d.ipv6)?;
            if !ids.insert(&d.id)
                || !ips.insert(IpAddr::V4(d.ipv4))
                || !ips.insert(IpAddr::V6(d.ipv6))
            {
                return Err(invalid("duplicate device or lease"));
            }
        }
        Ok(())
    }
}
pub async fn serve(config: Config, base: &Path) -> Result<()> {
    if !cfg!(target_os = "linux") {
        return Err(invalid("cloud server requires Linux"));
    }
    config.validate()?;
    let acceptor = TlsAcceptor::from(tls::server_config(
        &std::fs::read(base.join(&config.certificate))?,
        &std::fs::read(base.join(&config.private_key))?,
    )?);
    let mut tun_config = tun::Configuration::default();
    tun_config
        .tun_name("rl-server")
        .address((10, 77, 0, 1))
        .netmask((255, 255, 255, 0))
        .mtu(wire::MTU as u16)
        .up();
    let dev = Arc::new(tun::create_as_async(&tun_config).map_err(std::io::Error::other)?);
    crate::routes::command(
        "ip",
        &["-6", "addr", "add", "fd77::1/64", "dev", "rl-server"],
    )?;
    let sessions: Sessions = Arc::new(Mutex::new(HashMap::new()));
    let listener = TcpListener::bind(config.listen).await?;
    let config = Arc::new(config);
    let permits = Arc::new(Semaphore::new(256));
    let return_packets = async {
        let mut buf = vec![0; wire::MTU + 1];
        loop {
            let n = dev.recv(&mut buf).await?;
            if let Ok((_, dst)) = packet::addresses(&buf[..n]) {
                if let Some(tx) = sessions.lock().await.get(&dst) {
                    // Drop under congestion rather than let a slow device stall every client.
                    let _ = tx.try_send(buf[..n].to_vec());
                }
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), std::io::Error>(())
    };
    let accept = async {
        loop {
            let (tcp, _) = listener.accept().await?;
            let Ok(permit) = permits.clone().try_acquire_owned() else {
                drop(tcp);
                continue;
            };
            let acceptor = acceptor.clone();
            let config = config.clone();
            let sessions = sessions.clone();
            let dev = dev.clone();
            tokio::spawn(async move {
                let _permit = permit;
                let _ = session(tcp, acceptor, config, sessions, dev).await;
                // Deliberately do not log credentials, packet contents or destinations.
            });
        }
        #[allow(unreachable_code)]
        Ok::<(), std::io::Error>(())
    };
    println!("RouteLoom RL/1 listening on {}", config.listen);
    tokio::select! { r=return_packets=>r,r=accept=>r,r=tokio::signal::ctrl_c()=>r }
}
async fn session(
    tcp: tokio::net::TcpStream,
    acceptor: TlsAcceptor,
    config: Arc<Config>,
    sessions: Sessions,
    dev: Arc<tun::AsyncDevice>,
) -> Result<()> {
    tcp.set_nodelay(true)?;
    let (mut tls, device) = tokio::time::timeout(Duration::from_secs(10), async {
        let mut tls = acceptor.accept(tcp).await?;
        if tls.get_ref().1.alpn_protocol() != Some(wire::ALPN) {
            return Err(invalid("ALPN mismatch"));
        }
        let (kind, body) = wire::read_frame(&mut tls).await?;
        if kind != wire::HELLO {
            return Err(invalid("hello required"));
        }
        let hello: wire::Hello =
            serde_json::from_slice(&body).map_err(|_| invalid("invalid hello"))?;
        if hello.version != 1 {
            return Err(invalid("unsupported version"));
        }
        let device = config
            .devices
            .iter()
            .find(|d| d.authenticates(&hello.device, &hello.token))
            .cloned()
            .ok_or_else(|| invalid("authentication failed"))?;
        Ok((tls, device))
    })
    .await
    .map_err(|_| invalid("handshake timeout"))??;
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(64);
    {
        let mut all = sessions.lock().await;
        if all.contains_key(&device.ipv4.into()) {
            return Err(invalid("device already connected"));
        }
        all.insert(device.ipv4.into(), tx.clone());
        all.insert(device.ipv6.into(), tx);
    }
    let result=async {
        wire::write_frame(&mut tls,wire::WELCOME,&wire::json(&device.welcome())?).await?;
        let (mut reader,mut writer)=tokio::io::split(tls);
        let (pong_tx,mut pong_rx)=mpsc::channel::<()>(1);
        let incoming=async {
            loop {
                let (kind,p)=tokio::time::timeout(Duration::from_secs(70),wire::read_frame(&mut reader)).await
                    .map_err(|_|invalid("client idle timeout"))??;
                match kind {
                    wire::PING=>{let _=pong_tx.try_send(());},
                    wire::PACKET=>{
                        let (src,dst)=packet::addresses(&p)?;
                        if !device.owns(src) {return Err(invalid("spoofed source address"));}
                        if !packet::public_destination(dst) && !config.allow_private.iter().any(|net|net.contains(&dst)) {continue;}
                        dev.send(&p).await?;
                    },
                    _=>return Err(invalid("unexpected client frame")),
                }
            }
        };
        let outgoing=async {
            loop {
                tokio::select! {
                    p=rx.recv()=>if let Some(p)=p { wire::write_frame(&mut writer,wire::PACKET,&p).await?; } else {return Ok(());},
                    _=pong_rx.recv()=>wire::write_frame(&mut writer,wire::PONG,&[]).await?,
                }
            }
        };
        tokio::select! {r=incoming=>r,r=outgoing=>r}
    }.await;
    let mut all = sessions.lock().await;
    all.remove(&device.ipv4.into());
    all.remove(&device.ipv6.into());
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_leases_fail() {
        let d = Device {
            id: "one".into(),
            token_sha256: "ab".repeat(32),
            ipv4: "10.77.0.2".parse().unwrap(),
            ipv6: "fd77::2".parse().unwrap(),
        };
        let mut c = Config {
            listen: "127.0.0.1:9443".parse().unwrap(),
            certificate: "c".into(),
            private_key: "k".into(),
            devices: vec![d.clone()],
            allow_private: vec![],
        };
        assert!(c.validate().is_ok());
        c.devices.push(d);
        assert!(c.validate().is_err());
    }
}

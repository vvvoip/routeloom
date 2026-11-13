use crate::server::Config;
use routeloom_tunnel::{
    flow::{Kind, Reply, Request},
    invalid, packet, wire, Result,
};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    net::{TcpStream, UdpSocket},
    sync::Semaphore,
};
use tokio_rustls::server::TlsStream;
pub type Limits = Arc<Mutex<HashMap<String, Arc<Semaphore>>>>;
pub async fn serve(
    mut tls: TlsStream<TcpStream>,
    config: Arc<Config>,
    limits: Limits,
) -> Result<()> {
    let request: Request = tokio::time::timeout(Duration::from_secs(10), async {
        let (kind, data) = wire::read_frame(&mut tls).await?;
        if kind != wire::HELLO {
            return Err(invalid("hello required"));
        }
        let r: Request =
            serde_json::from_slice(&data).map_err(|_| invalid("invalid flow request"))?;
        r.validate()?;
        if !config
            .devices
            .iter()
            .any(|d| d.authenticates(&r.device, &r.token))
        {
            return Err(invalid("authentication failed"));
        }
        Ok(r)
    })
    .await
    .map_err(|_| invalid("handshake timeout"))??;
    let semaphore = limits
        .lock()
        .map_err(|_| invalid("session lock"))?
        .entry(request.device.clone())
        .or_insert_with(|| Arc::new(Semaphore::new(80)))
        .clone();
    let _permit = semaphore
        .try_acquire_owned()
        .map_err(|_| invalid("device connection limit"))?;
    let address = match request.kind {
        Kind::Tcp | Kind::Udp => {
            let ip: IpAddr = request
                .host
                .parse()
                .map_err(|_| invalid("invalid destination"))?;
            if !packet::public_destination(ip)
                && !config.allow_private.iter().any(|net| net.contains(&ip))
            {
                return Err(invalid("destination denied"));
            }
            Some(SocketAddr::new(ip, request.port))
        }
        _ => None,
    };
    // Resolve once; the client chooses a numeric address for the subsequent flow.
    let addresses = if request.kind == Kind::Resolve {
        tokio::time::timeout(
            Duration::from_secs(10),
            tokio::net::lookup_host((request.host.as_str(), 0)),
        )
        .await
        .map_err(|_| invalid("DNS timeout"))??
        .map(|a| a.ip())
        .take(16)
        .collect()
    } else {
        vec![]
    };
    let mut upstream = if request.kind == Kind::Tcp {
        Some(
            tokio::time::timeout(
                Duration::from_secs(15),
                TcpStream::connect(address.unwrap()),
            )
            .await
            .map_err(|_| invalid("connect timeout"))??,
        )
    } else {
        None
    };
    let udp = if request.kind == Kind::Udp {
        let addr = address.unwrap();
        let socket = UdpSocket::bind(if addr.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        })
        .await?;
        socket.connect(addr).await?;
        Some(socket)
    } else {
        None
    };
    wire::write_frame(
        &mut tls,
        wire::WELCOME,
        &wire::json(&Reply {
            version: 2,
            addresses,
        })?,
    )
    .await?;
    match request.kind {
        Kind::Resolve => Ok(()),
        Kind::Tcp => {
            // An absolute lifetime bounds abandoned connections; stream backpressure bounds memory.
            tokio::time::timeout(
                Duration::from_secs(86400),
                tokio::io::copy_bidirectional(&mut tls, upstream.as_mut().unwrap()),
            )
            .await
            .map_err(|_| invalid("flow lifetime exceeded"))??;
            Ok(())
        }
        Kind::Ping => loop {
            let (kind, _) =
                tokio::time::timeout(Duration::from_secs(45), wire::read_frame(&mut tls))
                    .await
                    .map_err(|_| invalid("heartbeat timeout"))??;
            if kind != wire::PING {
                return Err(invalid("expected ping"));
            }
            wire::write_frame(&mut tls, wire::PONG, &[]).await?;
        },
        Kind::Udp => {
            let socket = udp.unwrap();
            let (mut read, mut write) = tokio::io::split(tls);
            let upload = async {
                loop {
                    let (kind, data) =
                        tokio::time::timeout(Duration::from_secs(90), wire::read_frame(&mut read))
                            .await
                            .map_err(|_| invalid("UDP idle timeout"))??;
                    if kind != wire::PACKET {
                        return Err(invalid("expected datagram"));
                    }
                    socket.send(&data).await?;
                }
                #[allow(unreachable_code)]
                Ok::<(), std::io::Error>(())
            };
            let download = async {
                let mut buf = [0u8; 1281];
                loop {
                    let n = tokio::time::timeout(Duration::from_secs(90), socket.recv(&mut buf))
                        .await
                        .map_err(|_| invalid("UDP idle timeout"))??;
                    if n <= 1280 {
                        wire::write_frame(&mut write, wire::PACKET, &buf[..n]).await?;
                    }
                }
                #[allow(unreachable_code)]
                Ok::<(), std::io::Error>(())
            };
            tokio::select! {r=upload=>r,r=download=>r}
        }
    }
}

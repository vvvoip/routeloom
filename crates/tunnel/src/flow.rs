//! RL/2: a separately authenticated, bounded TLS connection per flow.
use crate::{config::Profile, invalid, tls, wire, Result};
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, time::Duration};
use tokio::net::TcpStream;
use tokio_rustls::{client::TlsStream, TlsConnector};
pub const ALPN: &[u8] = b"routeloom/2";
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Tcp,
    Udp,
    Resolve,
    Ping,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u8,
    pub device: String,
    pub token: String,
    pub kind: Kind,
    pub host: String,
    pub port: u16,
}
impl Request {
    pub fn validate(&self) -> Result<()> {
        crate::config::validate_identity(&self.device, &self.token)?;
        if self.version != 2 || self.host.len() > 253 {
            return Err(invalid("invalid flow request"));
        }
        match self.kind {
            Kind::Tcp | Kind::Udp => {
                if self.port == 0 || self.host.parse::<IpAddr>().is_err() {
                    return Err(invalid("numeric destination required"));
                }
            }
            Kind::Resolve => {
                if self.port != 0
                    || self.host.is_empty()
                    || !self.host.is_ascii()
                    || self.host.split('.').any(|s| {
                        s.is_empty()
                            || s.len() > 63
                            || s.starts_with('-')
                            || s.ends_with('-')
                            || !s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                    })
                {
                    return Err(invalid("invalid resolver request"));
                }
            }
            Kind::Ping => {
                if self.port != 0 || !self.host.is_empty() {
                    return Err(invalid("invalid ping"));
                }
            }
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub version: u8,
    pub addresses: Vec<IpAddr>,
}
pub async fn open(
    profile: &Profile,
    tcp: TcpStream,
    kind: Kind,
    host: &str,
    port: u16,
) -> Result<(TlsStream<TcpStream>, Vec<IpAddr>)> {
    let request = Request {
        version: 2,
        device: profile.device.clone(),
        token: profile.token.clone(),
        kind,
        host: host.into(),
        port,
    };
    request.validate()?;
    let config = tls::client_config_for(profile.ca_pem.as_bytes(), ALPN)?;
    let name = rustls::pki_types::ServerName::try_from(profile.server_name.clone())
        .map_err(|_| invalid("invalid TLS name"))?;
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut stream = TlsConnector::from(config).connect(name, tcp).await?;
        if stream.get_ref().1.alpn_protocol() != Some(ALPN) {
            return Err(invalid("RL/2 not supported"));
        }
        wire::write_frame(&mut stream, wire::HELLO, &wire::json(&request)?).await?;
        let (frame, data) = wire::read_frame(&mut stream).await?;
        let reply: Reply =
            serde_json::from_slice(&data).map_err(|_| invalid("invalid flow reply"))?;
        if frame != wire::WELCOME || reply.version != 2 || reply.addresses.len() > 16 {
            return Err(invalid("flow rejected"));
        }
        Ok((stream, reply.addresses))
    })
    .await
    .map_err(|_| invalid("flow handshake timed out"))?
}

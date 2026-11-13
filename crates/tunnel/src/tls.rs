use crate::{
    config::Profile,
    invalid,
    wire::{self, Hello, Welcome},
    Result,
};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, ServerName},
    ClientConfig, RootCertStore, ServerConfig,
};
use std::{io::BufReader, sync::Arc, time::Duration};
use tokio::net::TcpStream;
use tokio_rustls::{client::TlsStream, TlsConnector};

pub fn certificates(pem: &[u8]) -> Result<Vec<CertificateDer<'static>>> {
    let certs: Vec<_> = rustls_pemfile::certs(&mut BufReader::new(pem)).collect::<Result<_>>()?;
    if certs.is_empty() {
        return Err(invalid("empty certificate chain"));
    }
    Ok(certs)
}
pub fn private_key(pem: &[u8]) -> Result<PrivateKeyDer<'static>> {
    rustls_pemfile::private_key(&mut BufReader::new(pem))?
        .ok_or_else(|| invalid("missing private key"))
}
pub fn client_config(ca: &[u8]) -> Result<Arc<ClientConfig>> {
    client_config_for(ca, wire::ALPN)
}
pub fn client_config_for(ca: &[u8], alpn: &[u8]) -> Result<Arc<ClientConfig>> {
    let mut roots = RootCertStore::empty();
    for cert in certificates(ca)? {
        roots
            .add(cert)
            .map_err(|_| invalid("invalid CA certificate"))?;
    }
    let mut config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|_| invalid("TLS configuration"))?
            .with_root_certificates(roots)
            .with_no_client_auth();
    config.alpn_protocols = vec![alpn.to_vec()];
    config.enable_early_data = false;
    Ok(Arc::new(config))
}
pub fn server_config(cert: &[u8], key: &[u8]) -> Result<Arc<ServerConfig>> {
    let mut config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|_| invalid("TLS configuration"))?
            .with_no_client_auth()
            .with_single_cert(certificates(cert)?, private_key(key)?)
            .map_err(|_| invalid("certificate/key mismatch"))?;
    config.alpn_protocols = vec![crate::flow::ALPN.to_vec(), wire::ALPN.to_vec()];
    config.max_early_data_size = 0;
    Ok(Arc::new(config))
}
pub async fn connect(profile: &Profile) -> Result<TlsStream<TcpStream>> {
    profile.validate()?;
    let config = client_config(profile.ca_pem.as_bytes())?;
    let name = ServerName::try_from(profile.server_name.clone())
        .map_err(|_| invalid("invalid server name"))?;
    tokio::time::timeout(Duration::from_secs(15), async {
        let tcp = TcpStream::connect(profile.endpoint).await?;
        tcp.set_nodelay(true)?;
        let mut tls = TlsConnector::from(config).connect(name, tcp).await?;
        if tls.get_ref().1.alpn_protocol() != Some(wire::ALPN) {
            return Err(invalid("ALPN mismatch"));
        }
        let hello = Hello {
            version: 1,
            device: profile.device.clone(),
            token: profile.token.clone(),
        };
        wire::write_frame(&mut tls, wire::HELLO, &wire::json(&hello)?).await?;
        let (kind, body) = wire::read_frame(&mut tls).await?;
        let welcome: Welcome =
            serde_json::from_slice(&body).map_err(|_| invalid("invalid welcome"))?;
        if kind != wire::WELCOME || welcome != profile.welcome() {
            return Err(invalid("server rejected profile or lease mismatch"));
        }
        Ok(tls)
    })
    .await
    .map_err(|_| invalid("connection timed out"))?
}

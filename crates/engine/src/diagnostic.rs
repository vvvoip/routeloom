//! An explicit user-triggered probe through the same policy path as local proxy flows.
use crate::{invalid, Context, Result};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::{
    rustls::{self, pki_types::ServerName, RootCertStore},
    TlsConnector,
};
pub async fn run(c: Arc<Context>) {
    let started = Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(30), probe(&c))
        .await
        .unwrap_or_else(|_| Err(invalid("diagnostic timed out")));
    c.monitor
        .finish_diagnostic(result, started.elapsed().as_millis() as u64);
}
async fn probe(c: &Context) -> Result<(String, String)> {
    const HOST: &str = "api.ipify.org";
    let (stream, route) = c.tcp_with_route(HOST, 443, "diagnostic").await?;
    let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|_| invalid("TLS settings"))?
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let mut stream = TlsConnector::from(Arc::new(config))
        .connect(ServerName::try_from(HOST).unwrap(), stream)
        .await?;
    stream.write_all(b"GET / HTTP/1.1\r\nHost: api.ipify.org\r\nUser-Agent: RouteLoom-Diagnostic/1\r\nAccept: text/plain\r\nConnection: close\r\n\r\n").await?;
    let mut bytes = Vec::new();
    stream.take(16385).read_to_end(&mut bytes).await?;
    let ip = exit_address(&bytes)?;
    Ok((ip, route))
}
fn exit_address(bytes: &[u8]) -> Result<String> {
    if bytes.len() > 16384 {
        return Err(invalid("probe response too large"));
    }
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut response = httparse::Response::new(&mut headers);
    let start = match response
        .parse(bytes)
        .map_err(|_| invalid("invalid probe HTTP"))?
    {
        httparse::Status::Complete(n) => n,
        _ => return Err(invalid("incomplete probe HTTP")),
    };
    if response.code != Some(200) {
        return Err(invalid("probe HTTP status"));
    }
    let mut body = bytes[start..].to_vec();
    if let Some(header) = response
        .headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("Transfer-Encoding"))
    {
        if !header.value.eq_ignore_ascii_case(b"chunked") {
            return Err(invalid("unsupported transfer encoding"));
        }
        let mut input = body.as_slice();
        let mut decoded = Vec::new();
        loop {
            let (offset, size) =
                match httparse::parse_chunk_size(input).map_err(|_| invalid("invalid chunk"))? {
                    httparse::Status::Complete(v) => v,
                    _ => return Err(invalid("incomplete chunk")),
                };
            if size == 0 {
                break;
            }
            if size > 256 || input.len() < offset + size as usize + 2 {
                return Err(invalid("invalid chunk length"));
            }
            let end = offset + size as usize;
            if &input[end..end + 2] != b"\r\n" {
                return Err(invalid("invalid chunk ending"));
            }
            decoded.extend_from_slice(&input[offset..end]);
            if decoded.len() > 256 {
                return Err(invalid("probe body too large"));
            }
            input = &input[end + 2..];
        }
        body = decoded;
    } else if let Some(h) = response
        .headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("Content-Length"))
    {
        let len = std::str::from_utf8(h.value)
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| invalid("invalid length"))?;
        if body.len() != len {
            return Err(invalid("incomplete probe body"));
        }
    }
    if body.len() > 256 {
        return Err(invalid("probe body too large"));
    }
    let ip = std::str::from_utf8(&body)
        .map_err(|_| invalid("invalid IP response"))?
        .trim()
        .parse::<std::net::IpAddr>()
        .map_err(|_| invalid("invalid IP response"))?;
    Ok(ip.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_bounded_ip_responses_and_rejects_errors() {
        assert_eq!(
            exit_address(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\n1.2.3.4").unwrap(),
            "1.2.3.4"
        );
        assert_eq!(
            exit_address(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n7\r\n1.2.3.4\r\n0\r\n\r\n"
            )
            .unwrap(),
            "1.2.3.4"
        );
        for reply in [
            b"HTTP/1.1 502 Failed\r\n\r\n1.2.3.4".as_slice(),
            b"HTTP/1.1 200 OK\r\n\r\n<html>error</html>",
            b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\n1.2.3.4",
        ] {
            assert!(exit_address(reply).is_err());
        }
    }
}

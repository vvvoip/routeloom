//! Health scaffold only. It cannot proxy traffic and refuses non-loopback binding.
use std::{
    env,
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    process::ExitCode,
    time::Duration,
};

fn checked_address(value: &str) -> Result<SocketAddr, String> {
    let address: SocketAddr = value.parse().map_err(|_| "invalid socket address")?;
    if !address.ip().is_loopback() {
        return Err("health scaffold must bind to loopback".into());
    }
    Ok(address)
}
fn response(request: &[u8]) -> (&'static str, &'static str) {
    if request.starts_with(b"GET /healthz HTTP/1.1\r\n")
        || request.starts_with(b"GET /healthz HTTP/1.0\r\n")
    {
        (
            "200 OK",
            "{\"status\":\"development\",\"tunnel_ready\":false}\n",
        )
    } else {
        ("404 Not Found", "{\"error\":\"not_found\"}\n")
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.is_empty() || args == ["--help"] {
        println!("RouteLoom server scaffold / 服务端开发原型\nserve-health [127.0.0.1:9080]\nNo proxy transport is implemented. / 尚未实现代理传输。");
        return Ok(());
    }
    if args[0] != "serve-health" || args.len() > 2 {
        return Err("use: serve-health [127.0.0.1:9080]".into());
    }
    let address = checked_address(args.get(1).map(String::as_str).unwrap_or("127.0.0.1:9080"))?;
    let listener = TcpListener::bind(address)?;
    eprintln!(
        "Local health scaffold: http://{}/healthz (tunnel_ready=false)",
        listener.local_addr()?
    );
    for stream in listener.incoming() {
        let mut stream = stream?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        let mut buffer = [0_u8; 4096];
        let mut used = 0;
        let complete = loop {
            if used == buffer.len() {
                break false;
            }
            match stream.read(&mut buffer[used..]) {
                Ok(0) | Err(_) => break false,
                Ok(n) => {
                    used += n;
                    if buffer[..used].windows(4).any(|w| w == b"\r\n\r\n") {
                        break true;
                    }
                }
            }
        };
        if !complete {
            continue;
        }
        let (status, body) = response(&buffer[..used]);
        let reply = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{body}", body.len());
        let _ = stream.write_all(reply.as_bytes());
    }
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_public_listener() {
        for a in ["0.0.0.0:9080", "[::]:9080", "192.0.2.1:9080"] {
            assert!(checked_address(a).is_err());
        }
        assert!(checked_address("127.0.0.1:9080").is_ok());
        assert!(checked_address("[::1]:9080").is_ok());
    }
    #[test]
    fn health_does_not_claim_a_ready_tunnel() {
        let (code, body) = response(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(code, "200 OK");
        assert!(body.contains("\"tunnel_ready\":false"));
        assert_eq!(
            response(b"CONNECT example.org:443 HTTP/1.1\r\n\r\n").0,
            "404 Not Found"
        );
        assert_eq!(
            response(b"GET /healthz-extra HTTP/1.1\r\n\r\n").0,
            "404 Not Found"
        );
    }
}

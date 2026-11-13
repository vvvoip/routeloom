use crate::{invalid, Context, Result};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinSet,
};
pub async fn serve(c: Arc<Context>, listener: TcpListener) -> Result<()> {
    let mut tasks = JoinSet::new();
    loop {
        tokio::select! {
            item=listener.accept()=>{
                let (mut local,_)=item?;
                let Ok(permit)=c.permits.clone().try_acquire_owned()else{continue};
                let c=c.clone();
                tasks.spawn(async move{
                    let _permit=permit;
                    let _=tokio::time::timeout(Duration::from_secs(86400),async {
                        let mut header=Vec::new();
                        // Read only headers: no over-read that could lose a CONNECT ClientHello.
                        tokio::time::timeout(Duration::from_secs(10),async {
                            loop{
                                header.push(local.read_u8().await?);
                                if header.ends_with(b"\r\n\r\n"){break;}
                                if header.len()>=16384{return Err(invalid("proxy headers too large"));}
                            }
                            Ok::<(),std::io::Error>(())
                        }).await.map_err(|_|invalid("proxy header timeout"))??;
                        let mut headers=[httparse::EMPTY_HEADER;64];
                        let mut request=httparse::Request::new(&mut headers);
                        request.parse(&header).map_err(|_|invalid("invalid proxy request"))?;
                        let target=request.path.ok_or_else(||invalid("missing proxy target"))?;
                        let method=request.method.ok_or_else(||invalid("missing method"))?;
                        if method=="GET"&&target=="/proxy.pac" {
                            // Route every PAC request through this policy-aware listener. This
                            // preserves CIDR, port and block rules instead of silently losing them.
                            let pac=format!("function FindProxyForURL(url, host) {{ return 'PROXY 127.0.0.1:{}'; }}\n",c.profile.options.pac_port);
                            let response=format!("HTTP/1.1 200 OK\r\nContent-Type: application/x-ns-proxy-autoconfig\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",pac.len(),pac);
                            local.write_all(response.as_bytes()).await?;return Ok(());
                        }
                        let (host,port,path)=if method=="CONNECT"{
                            let (host,port)=authority(target,443)?;(host,port,None)
                        }else{
                            let rest=target.strip_prefix("http://").ok_or_else(||invalid("absolute HTTP URL required"))?;
                            let (auth,path)=rest.split_once('/').map(|(a,p)|(a,format!("/{p}"))).unwrap_or((rest,"/".into()));
                            let (host,port)=authority(auth,80)?;(host,port,Some(path))
                        };
                        let mut remote=match c.tcp(&host,port,"local-proxy").await {
                            Ok(s)=>s,
                            Err(_)=>{local.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;return Ok(());}
                        };
                        if let Some(path)=path {
                            let mut rewritten=format!("{method} {path} HTTP/1.1\r\n");
                            for h in request.headers.iter(){
                                if ["proxy-authorization","proxy-connection","connection","host"].iter().any(|v|h.name.eq_ignore_ascii_case(v)){continue;}
                                let value=std::str::from_utf8(h.value).map_err(|_|invalid("invalid header"))?;
                                rewritten.push_str(&format!("{}: {}\r\n",h.name,value));
                            }
                            let host_header=if host.contains(':'){format!("[{host}]")}else{host.clone()};
                            rewritten.push_str(&format!("Host: {host_header}:{port}\r\nConnection: close\r\n\r\n"));
                            remote.write_all(rewritten.as_bytes()).await?;
                        }else{local.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;}
                        tokio::select!{
                            r=tokio::io::copy_bidirectional(&mut local,&mut remote)=>{r?;},
                            _=c.wait_offline()=>{}
                        }
                        Ok::<(),std::io::Error>(())
                    }).await;
                });
            },
            _=tasks.join_next(),if !tasks.is_empty()=>{}
        }
    }
}
fn authority(value: &str, default: u16) -> Result<(String, u16)> {
    if value.contains('@') || value.contains('#') || value.contains('?') {
        return Err(invalid("invalid authority"));
    }
    let (host, port) = if let Some(rest) = value.strip_prefix('[') {
        let (host, tail) = rest
            .split_once(']')
            .ok_or_else(|| invalid("invalid IPv6 authority"))?;
        (
            host,
            if tail.is_empty() {
                default
            } else {
                tail.strip_prefix(':')
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| invalid("invalid port"))?
            },
        )
    } else if let Some((host, port)) = value.rsplit_once(':') {
        (host, port.parse().map_err(|_| invalid("invalid port"))?)
    } else {
        (value, default)
    };
    routeloom_core::Flow::new(host, None, port, routeloom_core::Network::Tcp)
        .map_err(|_| invalid("invalid host"))?;
    Ok((host.to_ascii_lowercase(), port))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_authorities() {
        assert_eq!(authority("[::1]:80", 443).unwrap(), ("::1".into(), 80));
        assert!(authority("a@b:443", 443).is_err());
        assert!(authority("a:0", 443).is_err());
    }
}

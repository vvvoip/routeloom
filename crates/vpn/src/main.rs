mod routes;
mod server;
use routeloom_tunnel::{config::Profile, invalid, packet, tls, wire, Result};
use std::{path::Path, sync::Arc, time::Duration};
use tun::AbstractDevice;

fn read_config<T: serde::de::DeserializeOwned>(path: &str) -> Result<T> {
    let data = std::fs::read(path)?;
    if data.len() > 65536 {
        return Err(invalid("configuration too large"));
    }
    serde_json::from_slice(&data).map_err(|_| invalid("invalid configuration"))
}
#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    let result = match args.get(1).map(String::as_str) {
        Some("connect-stdio") if args.len() == 2 => connect_stdio().await,
        Some("connect") if args.len() == 3 => match read_config::<Profile>(&args[2]) {
            Ok(profile) => connect(profile).await,
            Err(e) => Err(e),
        },
        Some("serve") if args.len() == 3 => match read_config::<server::Config>(&args[2]) {
            Ok(config) => {
                server::serve(
                    config,
                    Path::new(&args[2]).parent().unwrap_or(Path::new(".")),
                )
                .await
            }
            Err(e) => Err(e),
        },
        _ => Err(invalid(
            "Usage: routeloom-vpn connect PROFILE.json | serve SERVER.json",
        )),
    };
    if let Err(e) = result {
        eprintln!("RouteLoom: {e}");
        std::process::exit(1);
    }
}
async fn connect(profile: Profile) -> Result<()> {
    let tls = tls::connect(&profile).await?;
    let mut config = tun::Configuration::default();
    config
        .address(profile.ipv4)
        .netmask((255, 255, 255, 0))
        .mtu(wire::MTU as u16)
        .up();
    #[cfg(target_os = "linux")]
    config.tun_name(format!("rl{}", std::process::id()));
    #[cfg(target_os = "windows")]
    {
        config.tun_name(format!("RouteLoom-{}", std::process::id()));
        let dll = std::env::current_exe()?.with_file_name("wintun.dll");
        config.platform_config(|p| {
            p.wintun_file(dll);
            p.wait_for_interfaces(true, true, Duration::from_secs(10));
        });
    }
    let dev = Arc::new(tun::create_as_async(&config).map_err(std::io::Error::other)?);
    let name = dev.tun_name().map_err(std::io::Error::other)?;
    let _routes = routes::Routes::apply(&name, &profile)?;
    println!("{{\"state\":\"connected\"}}");
    let (mut reader, mut writer) = tokio::io::split(tls);
    let outgoing = async {
        let mut buf = vec![0; wire::MTU + 1];
        let mut heartbeat = tokio::time::interval(Duration::from_secs(20));
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                n=dev.recv(&mut buf)=> {
                    let n=n?;
                    let (src,_)=packet::addresses(&buf[..n])?;
                    if src!=profile.ipv4 && src!=profile.ipv6 {continue;}
                    wire::write_frame(&mut writer,wire::PACKET,&buf[..n]).await?;
                },
                _=heartbeat.tick()=>wire::write_frame(&mut writer,wire::PING,&[]).await?,
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), std::io::Error>(())
    };
    let incoming = async {
        loop {
            let (kind, p) =
                tokio::time::timeout(Duration::from_secs(70), wire::read_frame(&mut reader))
                    .await
                    .map_err(|_| invalid("server idle timeout"))??;
            match kind {
                wire::PACKET => {
                    let (_, dst) = packet::addresses(&p)?;
                    if dst != profile.ipv4 && dst != profile.ipv6 {
                        return Err(invalid("incorrect packet destination"));
                    }
                    dev.send(&p).await?;
                }
                wire::PONG => {}
                _ => return Err(invalid("unexpected frame")),
            }
        }
    };
    tokio::select! {
        r=outgoing=>r,
        r=incoming=>r,
        r=tokio::signal::ctrl_c()=>r,
    }
}

async fn connect_stdio() -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt};
    let mut input = tokio::io::BufReader::new(tokio::io::stdin().take(65537));
    let mut raw = String::new();
    input.read_line(&mut raw).await?;
    if raw.len() > 65536 {
        return Err(invalid("profile too large"));
    }
    let profile: Profile = serde_json::from_str(&raw).map_err(|_| invalid("invalid profile"))?;
    raw.clear();
    tokio::select! {r=connect(profile)=>r, _=input.read_line(&mut raw)=>Ok(())}
}

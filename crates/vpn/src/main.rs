mod flow_server;
mod guard;
mod routes;
mod server;
#[cfg(target_os = "windows")]
mod updater;
use routeloom_tunnel::{config::Profile, invalid, wire, Result};
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
        #[cfg(target_os = "windows")]
        Some("install-update") if args.len() == 2 => updater::run(),
        #[cfg(target_os = "windows")]
        Some("guard-test") if args.len() == 2 => guard::self_test(),
        #[cfg(target_os = "windows")]
        Some("guard-enable") if args.len() == 3 => guard::enable(&args[2]),
        Some("guard-rules") if args.len() == 3 => match guard::linux_rules(&args[2]) {
            Ok(rules) => {
                print!("{rules}");
                Ok(())
            }
            Err(e) => Err(e),
        },
        Some("unlock") if args.len() == 2 => guard::disable(),
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
    profile.validate()?;
    let protector = routeloom_engine::socket::desktop_protector(profile.endpoint);
    let mut engine = routeloom_engine::start(profile.clone(), protector)?;
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
    if profile.options.kill_switch {
        guard::enable(&name)?;
    }
    println!("{{\"state\":\"connecting\"}}");
    let mut last = 0;
    let mut status = tokio::time::interval(Duration::from_millis(500));
    let mut packet = vec![0; 1281];
    loop {
        tokio::select! {
            n=dev.recv(&mut packet)=>{let n=n?;let _=engine.input.try_send(packet[..n].to_vec());},
            p=engine.output.recv()=>{if let Some(p)=p{dev.send(&p).await?;}else{return Err(invalid("engine stopped"));}},
            _=status.tick()=>{
                let state=engine.state.load(std::sync::atomic::Ordering::Acquire);
                if state!=last {
                    println!("{{\"state\":\"{}\"}}",if state==1{"connected"}else if state==0{"reconnecting"}else{"blocked"});last=state;
                }
            },
            r=tokio::signal::ctrl_c()=>return r,
        }
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
    if raw.trim() == "unlock" {
        return guard::disable();
    }
    let profile: Profile = serde_json::from_str(&raw).map_err(|_| invalid("invalid profile"))?;
    raw.clear();
    tokio::select! {r=connect(profile)=>r, _=input.read_line(&mut raw)=>Ok(())}
}

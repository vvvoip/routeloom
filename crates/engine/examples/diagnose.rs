//! Developer probe: exercises policy routing without creating a VPN interface.
use routeloom_tunnel::config::{Profile, RoutingMode};
use std::{sync::atomic::Ordering, time::Duration};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err("Usage: diagnose PRIVATE_PROFILE.json auto|proxy|direct".into());
    }
    let mut profile: Profile = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    profile.options.mode = match args[2].as_str() {
        "auto" => RoutingMode::Auto,
        "proxy" => RoutingMode::Proxy,
        "direct" => RoutingMode::Direct,
        _ => return Err("Invalid mode".into()),
    };
    profile.options.pac_port = 0;
    let directory =
        std::env::temp_dir().join(format!("routeloom-diagnostic-{}", rand::random::<u64>()));
    let protect = routeloom_engine::socket::desktop_protector(profile.endpoint);
    let mut engine = routeloom_engine::start_in(profile, protect, directory.clone())?;
    let result = tokio::time::timeout(Duration::from_secs(55), async {
        while engine.state.load(Ordering::Acquire) != 1 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        engine
            .commands
            .send(())
            .await
            .map_err(std::io::Error::other)?;
        loop {
            let diagnostic = engine.monitor.snapshot().diagnostic;
            if diagnostic.status == "success" || diagnostic.status == "failed" {
                println!(
                    "{}",
                    serde_json::to_string(&diagnostic).map_err(std::io::Error::other)?
                );
                if diagnostic.status == "failed" {
                    return Err(std::io::Error::other("Connection probe failed"));
                }
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Ok::<(), std::io::Error>(())
    })
    .await;
    engine.shutdown().await;
    drop(engine);
    std::fs::remove_dir_all(directory)?;
    result??;
    Ok(())
}

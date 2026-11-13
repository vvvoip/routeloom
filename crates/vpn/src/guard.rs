//! Owned persistent OS filters. Disconnect never removes protection; unlock is explicit.
use routeloom_tunnel::{invalid, Result};
#[cfg(target_os = "windows")]
#[path = "guard_windows.rs"]
mod platform;
#[cfg(target_os = "windows")]
pub use platform::{disable, enable};
#[cfg(target_os = "linux")]
pub fn enable(_name: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let rules = linux_rules();
    let dir = std::path::Path::new("/etc/routeloom-client");
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    let file = dir.join("guard.nft");
    std::fs::write(&file, rules)?;
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))?;
    std::fs::write("/etc/systemd/system/routeloom-guard.service", "[Unit]\nDescription=RouteLoom persistent traffic guard\nDefaultDependencies=no\nAfter=local-fs.target\nBefore=network-pre.target\nWants=network-pre.target\n[Service]\nType=oneshot\nExecStart=/usr/sbin/nft -f /etc/routeloom-client/guard.nft\nRemainAfterExit=yes\n[Install]\nWantedBy=network-pre.target\n")?;
    crate::routes::command("systemctl", &["daemon-reload"])?;
    crate::routes::command("systemctl", &["enable", "routeloom-guard.service"])?;
    // nft applies the entire file as one atomic transaction.
    crate::routes::command("/usr/sbin/nft", &["-f", file.to_str().unwrap()])
}
#[cfg(target_os = "linux")]
pub fn disable() -> Result<()> {
    // Disarm boot persistence only after the current owned table is removed.
    let exists = std::process::Command::new("/usr/sbin/nft")
        .args(["list", "table", "inet", "routeloom_guard"])
        .output()?
        .status
        .success();
    if exists {
        crate::routes::command(
            "/usr/sbin/nft",
            &["delete", "table", "inet", "routeloom_guard"],
        )?;
    }
    if std::path::Path::new("/etc/systemd/system/routeloom-guard.service").exists() {
        crate::routes::command("systemctl", &["disable", "routeloom-guard.service"])?;
        std::fs::remove_file("/etc/systemd/system/routeloom-guard.service")?;
        crate::routes::command("systemctl", &["daemon-reload"])?;
    }
    match std::fs::remove_file("/etc/routeloom-client/guard.nft") {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn enable(_: &str) -> Result<()> {
    Err(invalid("use native system VPN protection"))
}
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn disable() -> Result<()> {
    Err(invalid("use native system VPN protection"))
}
#[allow(dead_code)]
pub fn linux_rules() -> &'static str {
    "add table inet routeloom_guard\nflush table inet routeloom_guard\nadd chain inet routeloom_guard output { type filter hook output priority -50; policy drop; }\nadd rule inet routeloom_guard output oifname \"lo\" accept\nadd rule inet routeloom_guard output meta mark 0x524c accept\nadd rule inet routeloom_guard output oifname \"rl*\" accept\nadd rule inet routeloom_guard output udp sport 68 udp dport 67 accept\nadd rule inet routeloom_guard output udp sport 546 udp dport 547 accept\nadd rule inet routeloom_guard output icmpv6 type { nd-router-solicit, nd-router-advert, nd-neighbor-solicit, nd-neighbor-advert } accept\n"
}

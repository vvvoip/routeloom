//! Owned route changes, rolled back in reverse order on failure or normal exit.
use ipnet::IpNet;
use routeloom_tunnel::{config::Profile, invalid, Result};
use std::{net::IpAddr, process::Command};
pub fn command(program: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(program).args(args).output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "{program} failed; check privileges and VPN prerequisites"
        )));
    }
    Ok(())
}
pub struct Routes {
    undo: Vec<(String, Vec<String>)>,
}
impl Routes {
    fn add(&mut self, program: &str, args: Vec<String>, undo: Vec<String>) -> Result<()> {
        command(
            program,
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
        )?;
        self.undo.push((program.into(), undo));
        Ok(())
    }
    pub fn apply(name: &str, p: &Profile) -> Result<Self> {
        let mut owned = Self { undo: vec![] };
        let mut included = p.routes.clone();
        included.push("198.18.0.0/16".parse().unwrap());
        included.push("fdfe::/112".parse().unwrap());
        let routes = capture_routes(&included, p.endpoint.ip())?;
        #[cfg(target_os = "linux")]
        {
            owned.add(
                "ip",
                vec![
                    "-6".into(),
                    "addr".into(),
                    "add".into(),
                    format!("{}/64", p.ipv6),
                    "dev".into(),
                    name.into(),
                ],
                vec![
                    "-6".into(),
                    "addr".into(),
                    "del".into(),
                    format!("{}/64", p.ipv6),
                    "dev".into(),
                    name.into(),
                ],
            )?;
            for route in routes {
                let family = if route.addr().is_ipv6() { "-6" } else { "-4" };
                owned.add(
                    "ip",
                    vec![
                        family.into(),
                        "route".into(),
                        "add".into(),
                        route.to_string(),
                        "dev".into(),
                        name.into(),
                        "metric".into(),
                        "5".into(),
                    ],
                    vec![
                        family.into(),
                        "route".into(),
                        "del".into(),
                        route.to_string(),
                        "dev".into(),
                        name.into(),
                        "metric".into(),
                        "5".into(),
                    ],
                )?;
            }
            let mut dns = vec!["dns".into(), name.into()];
            dns.extend(p.dns.iter().map(ToString::to_string));
            owned.add("resolvectl", dns, vec!["revert".into(), name.into()])?;
            command("resolvectl", &["domain", name, "~."])?;
        }
        #[cfg(target_os = "windows")]
        {
            owned.add(
                "netsh",
                vec![
                    "interface".into(),
                    "ipv6".into(),
                    "add".into(),
                    "address".into(),
                    name.into(),
                    p.ipv6.to_string(),
                ],
                vec![
                    "interface".into(),
                    "ipv6".into(),
                    "delete".into(),
                    "address".into(),
                    name.into(),
                    p.ipv6.to_string(),
                ],
            )?;
            for route in routes {
                let family = if route.addr().is_ipv6() {
                    "ipv6"
                } else {
                    "ipv4"
                };
                owned.add(
                    "netsh",
                    vec![
                        "interface".into(),
                        family.into(),
                        "add".into(),
                        "route".into(),
                        route.to_string(),
                        name.into(),
                        "store=active".into(),
                        "metric=5".into(),
                    ],
                    vec![
                        "interface".into(),
                        family.into(),
                        "delete".into(),
                        "route".into(),
                        route.to_string(),
                        name.into(),
                        "store=active".into(),
                    ],
                )?;
            }
            for ip in &p.dns {
                let family = if ip.is_ipv6() { "ipv6" } else { "ipv4" };
                owned.add(
                    "netsh",
                    vec![
                        "interface".into(),
                        family.into(),
                        "add".into(),
                        "dnsservers".into(),
                        name.into(),
                        format!("address={ip}"),
                        "validate=no".into(),
                    ],
                    vec![
                        "interface".into(),
                        family.into(),
                        "delete".into(),
                        "dnsservers".into(),
                        name.into(),
                        format!("address={ip}"),
                        "validate=no".into(),
                    ],
                )?;
            }
        }
        #[cfg(target_os = "macos")]
        {
            let _ = (&mut owned, name, p, routes);
            return Err(invalid(
                "Use the signed macOS Network Extension client for VPN capture",
            ));
        }
        #[allow(unreachable_code)]
        Ok(owned)
    }
}
impl Drop for Routes {
    fn drop(&mut self) {
        for (program, args) in self.undo.iter().rev() {
            if command(
                program,
                &args.iter().map(String::as_str).collect::<Vec<_>>(),
            )
            .is_err()
            {
                eprintln!(
                    "RouteLoom: route cleanup failed; inspect the temporary tunnel interface"
                );
            }
        }
    }
}
/// Subtract the endpoint host from every route, keeping the physical path alive.
/// No DNS bootstrap, overwritten default route or mutable gateway snapshot needed.
pub fn capture_routes(routes: &[IpNet], endpoint: IpAddr) -> Result<Vec<IpNet>> {
    fn subtract(net: IpNet, ip: IpAddr, out: &mut Vec<IpNet>) {
        if !net.contains(&ip) {
            out.push(net);
            return;
        }
        let bits = if ip.is_ipv4() { 32 } else { 128 };
        if net.prefix_len() == bits {
            return;
        }
        let children = net.subnets(net.prefix_len() + 1).expect("valid prefix");
        for child in children {
            subtract(child, ip, out);
        }
    }
    let mut out = vec![];
    for r in ipnet::IpNet::aggregate(&routes.to_vec()) {
        // Two /1 routes outrank a physical /0 regardless of interface metrics.
        if r.prefix_len() == 0 {
            for half in r.subnets(1).expect("valid default prefix") {
                subtract(half, endpoint, &mut out);
            }
        } else {
            subtract(r, endpoint, &mut out);
        }
    }
    if out.len() > 256 {
        return Err(invalid("too many capture routes"));
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoint_is_never_recaptured() {
        for endpoint in ["203.0.113.7", "2001:db8::7"] {
            let ip = endpoint.parse().unwrap();
            let routes =
                capture_routes(&["0.0.0.0/0".parse().unwrap(), "::/0".parse().unwrap()], ip)
                    .unwrap();
            assert!(!routes.iter().any(|r| r.contains(&ip)));
            assert!(routes.iter().all(|r| r.prefix_len() > 0));
            for other in ["1.1.1.1", "10.1.2.3", "2606:4700::1111"] {
                assert!(routes
                    .iter()
                    .any(|r| r.contains(&other.parse::<IpAddr>().unwrap())));
            }
        }
    }
}

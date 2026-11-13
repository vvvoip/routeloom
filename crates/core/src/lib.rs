//! Deterministic policy evaluation. No sockets, credentials, DNS lookups or OS changes.
use std::{
    fmt,
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyError(pub String);
impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for PolicyError {}
fn invalid(message: impl Into<String>) -> PolicyError {
    PolicyError(message.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    Tcp,
    Udp,
}
impl FromStr for Network {
    type Err = PolicyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "tcp" => Ok(Self::Tcp),
            "udp" => Ok(Self::Udp),
            _ => Err(invalid("network must be tcp or udp")),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Direct,
    Block,
    Proxy(String),
}
impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Direct => f.write_str("direct"),
            Self::Block => f.write_str("block"),
            Self::Proxy(name) => write!(f, "proxy:{name}"),
        }
    }
}
impl FromStr for Action {
    type Err = PolicyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "direct" => Ok(Self::Direct),
            "block" => Ok(Self::Block),
            _ => value
                .strip_prefix("proxy:")
                .filter(|n| {
                    !n.is_empty()
                        && n.len() <= 64
                        && n.bytes()
                            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
                })
                .map(|n| Self::Proxy(n.into()))
                .ok_or_else(|| invalid("action must be direct, block, or proxy:<name>")),
        }
    }
}
fn canonical_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        _ => ip,
    }
}
fn domain(value: &str) -> Result<String, PolicyError> {
    let value = value
        .strip_suffix('.')
        .unwrap_or(value)
        .to_ascii_lowercase();
    if value.len() > 253
        || value.is_empty()
        || !value.is_ascii()
        || value.parse::<IpAddr>().is_ok()
        || value.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        })
    {
        return Err(invalid(
            "expected a DNS hostname (use ASCII/Punycode for international names)",
        ));
    }
    Ok(value)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cidr {
    network: IpAddr,
    prefix: u8,
}
impl FromStr for Cidr {
    type Err = PolicyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (address, prefix) = value
            .split_once('/')
            .ok_or_else(|| invalid("CIDR requires an address and prefix"))?;
        let network: IpAddr = address
            .parse()
            .map_err(|_| invalid("invalid CIDR address"))?;
        let prefix: u8 = prefix.parse().map_err(|_| invalid("invalid CIDR prefix"))?;
        if prefix > if network.is_ipv4() { 32 } else { 128 } {
            return Err(invalid("CIDR prefix is out of range"));
        }
        if matches!(network, IpAddr::V6(v6) if v6.to_ipv4_mapped().is_some()) {
            return Err(invalid("write IPv4-mapped CIDRs as IPv4 CIDRs"));
        }
        Ok(Self { network, prefix })
    }
}
impl Cidr {
    pub fn contains(&self, address: IpAddr) -> bool {
        match (self.network, canonical_ip(address)) {
            (IpAddr::V4(a), IpAddr::V4(b)) => {
                let mask = u32::MAX
                    .checked_shl(32 - u32::from(self.prefix))
                    .unwrap_or(0);
                u32::from(a) & mask == u32::from(b) & mask
            }
            (IpAddr::V6(a), IpAddr::V6(b)) => {
                let mask = u128::MAX
                    .checked_shl(128 - u32::from(self.prefix))
                    .unwrap_or(0);
                u128::from(a) & mask == u128::from(b) & mask
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Flow {
    domain: Option<String>,
    address: Option<IpAddr>,
    port: u16,
    network: Network,
}
impl Flow {
    /// The caller supplies the resolved IP. Evaluation never performs DNS.
    pub fn new(
        host: &str,
        resolved: Option<IpAddr>,
        port: u16,
        network: Network,
    ) -> Result<Self, PolicyError> {
        if port == 0 {
            return Err(invalid("destination port cannot be zero"));
        }
        let (domain, address) = match host.parse::<IpAddr>() {
            Ok(ip) => {
                let ip = canonical_ip(ip);
                if resolved.is_some_and(|other| canonical_ip(other) != ip) {
                    return Err(invalid("literal and resolved addresses disagree"));
                }
                (None, Some(ip))
            }
            Err(_) => (Some(domain(host)?), resolved.map(canonical_ip)),
        };
        Ok(Self {
            domain,
            address,
            port,
            network,
        })
    }
}
#[derive(Debug, Clone)]
enum Matcher {
    Domain(String),
    Suffix(String),
    Cidr(Cidr),
    Port(u16),
    Network(Network),
}
impl Matcher {
    fn matches(&self, flow: &Flow) -> bool {
        match self {
            Self::Domain(wanted) => flow.domain.as_ref() == Some(wanted),
            Self::Suffix(wanted) => flow.domain.as_ref().is_some_and(|host| {
                host == wanted
                    || host
                        .strip_suffix(wanted)
                        .is_some_and(|prefix| prefix.ends_with('.'))
            }),
            Self::Cidr(net) => flow.address.is_some_and(|ip| net.contains(ip)),
            Self::Port(port) => flow.port == *port,
            Self::Network(network) => flow.network == *network,
        }
    }
}
#[derive(Debug, Clone)]
struct Rule {
    matcher: Matcher,
    action: Action,
    line: usize,
}
#[derive(Debug, Clone)]
pub struct Policy {
    rules: Vec<Rule>,
    fallback: Action,
    fallback_line: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub action: Action,
    pub line: usize,
    pub is_fallback: bool,
}

impl Policy {
    /// Bounded v0 rules syntax. The final action must be explicit and last.
    pub fn parse(input: &str) -> Result<Self, PolicyError> {
        if input.len() > 1_048_576 {
            return Err(invalid("policy exceeds 1 MiB"));
        }
        let mut rules = Vec::new();
        let mut fallback = None;
        for (index, raw) in input.lines().enumerate() {
            let line = index + 1;
            let parts: Vec<_> = raw
                .split('#')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .collect();
            if parts.is_empty() {
                continue;
            }
            let parse_line = || -> Result<_, PolicyError> {
                if fallback.is_some() {
                    return Err(invalid("final must be the last rule"));
                }
                if parts[0] == "final" {
                    if parts.len() != 2 {
                        return Err(invalid("expected: final <action>"));
                    }
                    return Ok((None, parts[1].parse()?));
                }
                if parts.len() != 3 {
                    return Err(invalid("expected: <matcher> <value> <action>"));
                }
                let matcher = match parts[0] {
                    "domain" => Matcher::Domain(domain(parts[1])?),
                    "domain-suffix" => Matcher::Suffix(domain(parts[1])?),
                    "cidr" => Matcher::Cidr(parts[1].parse()?),
                    "port" => Matcher::Port(parts[1].parse::<u16>().ok().filter(|p| *p > 0).ok_or_else(|| invalid("invalid destination port"))?),
                    "network" => Matcher::Network(parts[1].parse()?),
                    _ => return Err(invalid("unsupported matcher; expected domain, domain-suffix, cidr, port, or network")),
                };
                Ok((Some(matcher), parts[2].parse()?))
            };
            let (matcher, action) =
                parse_line().map_err(|e| invalid(format!("line {line}: {e}")))?;
            match matcher {
                Some(matcher) => {
                    rules.push(Rule {
                        matcher,
                        action,
                        line,
                    });
                    if rules.len() > 10_000 {
                        return Err(invalid("policy exceeds 10000 rules"));
                    }
                }
                None => fallback = Some((action, line)),
            }
        }
        let (fallback, fallback_line) = fallback.ok_or_else(|| invalid("missing final action"))?;
        Ok(Self {
            rules,
            fallback,
            fallback_line,
        })
    }
    pub fn validate_default_group(&self) -> Result<(), PolicyError> {
        for action in self
            .rules
            .iter()
            .map(|r| &r.action)
            .chain(std::iter::once(&self.fallback))
        {
            if matches!(action, Action::Proxy(name) if name!="default") {
                return Err(invalid("only proxy:default is configured"));
            }
        }
        Ok(())
    }
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
    pub fn evaluate(&self, flow: &Flow) -> Decision {
        self.rules
            .iter()
            .find(|rule| rule.matcher.matches(flow))
            .map(|rule| Decision {
                action: rule.action.clone(),
                line: rule.line,
                is_fallback: false,
            })
            .unwrap_or_else(|| Decision {
                action: self.fallback.clone(),
                line: self.fallback_line,
                is_fallback: true,
            })
    }
    /// Exports only the lossless domain-only subset. Never drops unsupported rules.
    pub fn to_pac(&self, local_proxy: SocketAddr) -> Result<String, PolicyError> {
        if !local_proxy.ip().is_loopback() || !local_proxy.is_ipv4() || local_proxy.port() == 0 {
            return Err(invalid(
                "PAC endpoint must be IPv4 loopback with a nonzero port",
            ));
        }
        let pac_action = |action: &Action| -> Result<String, PolicyError> {
            match action {
                Action::Direct => Ok("DIRECT".into()),
                Action::Proxy(name) if name == "default" => Ok(format!("PROXY {local_proxy}")),
                _ => Err(invalid("PAC supports direct and proxy:default only; blocking and other groups cannot be represented")),
            }
        };
        let mut output = String::from("// Generated by RouteLoom. No implicit DIRECT fallback.\nfunction FindProxyForURL(url, host) {\n  host = host.toLowerCase().replace(/\\.$/, '');\n");
        for rule in &self.rules {
            let predicate = match &rule.matcher {
                Matcher::Domain(name) => format!("host === '{name}'"),
                Matcher::Suffix(name) => {
                    format!("(host === '{name}' || dnsDomainIs(host, '.{name}'))")
                }
                _ => {
                    return Err(invalid(format!(
                        "line {}: PAC cannot preserve this matcher",
                        rule.line
                    )))
                }
            };
            output.push_str(&format!(
                "  if ({predicate}) return '{}';\n",
                pac_action(&rule.action)?
            ));
        }
        output.push_str(&format!(
            "  return '{}';\n}}\n",
            pac_action(&self.fallback)?
        ));
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_china_lan_policy_routes_destinations_and_overseas_exceptions() {
        let text = include_str!("../../../apps/client/assets/china-lan.rules");
        assert!(text.len() <= 16384);
        let policy = Policy::parse(text).unwrap();
        policy.validate_default_group().unwrap();
        for network in [Network::Tcp, Network::Udp] {
            for host in [
                "192.168.1.1",
                "10.2.3.4",
                "fd12::1",
                "fe80::1",
                "::1",
                "printer.local",
                "www.gov.cn",
                "www.baidu.com",
                "www.qq.com",
                "www.bilibili.com",
            ] {
                let flow = Flow::new(host, None, 443, network).unwrap();
                assert_eq!(policy.evaluate(&flow).action, Action::Direct, "{host}");
            }
            for host in [
                "www.google.com",
                "api.ipify.org",
                "wetv.qq.com",
                "hk.weibo.com",
                "c.mi.com",
                "baidu.com.evil.test",
                "notbaidu.com",
                "8.8.8.8",
                "2606:4700:4700::1111",
            ] {
                let flow = Flow::new(host, None, 443, network).unwrap();
                assert_eq!(
                    policy.evaluate(&flow).action,
                    Action::Proxy("default".into()),
                    "{host}"
                );
            }
        }
    }
    fn flow(host: &str) -> Flow {
        Flow::new(host, None, 443, Network::Tcp).unwrap()
    }
    #[test]
    fn suffix_boundary() {
        let p = Policy::parse("domain-suffix example.com direct\nfinal block").unwrap();
        for h in ["example.com", "api.example.com", "API.EXAMPLE.COM."] {
            assert_eq!(p.evaluate(&flow(h)).action, Action::Direct);
        }
        for h in ["notexample.com", "example.com.evil.test"] {
            assert_eq!(p.evaluate(&flow(h)).action, Action::Block);
        }
    }
    #[test]
    fn first_match_wins() {
        let p = Policy::parse("domain blocked.example.com block\ndomain-suffix example.com direct\nfinal proxy:default").unwrap();
        let d = p.evaluate(&flow("blocked.example.com"));
        assert_eq!(d.action, Action::Block);
        assert_eq!(d.line, 1);
        assert!(!d.is_fallback);
    }
    #[test]
    fn ipv4_boundaries_and_mapped_addresses() {
        let net: Cidr = "10.1.2.7/24".parse().unwrap();
        for ip in ["10.1.2.0", "10.1.2.255", "::ffff:10.1.2.42"] {
            assert!(net.contains(ip.parse().unwrap()));
        }
        for ip in ["10.1.3.0", "2001:db8::1"] {
            assert!(!net.contains(ip.parse().unwrap()));
        }
    }
    #[test]
    fn ipv6_boundaries_and_zero_prefixes() {
        let net: Cidr = "2001:db8::/32".parse().unwrap();
        assert!(net.contains("2001:db8:ffff::1".parse().unwrap()));
        assert!(!net.contains("2001:db9::1".parse().unwrap()));
        assert!("::/0"
            .parse::<Cidr>()
            .unwrap()
            .contains("2001:db8::1".parse().unwrap()));
        assert!("0.0.0.0/0"
            .parse::<Cidr>()
            .unwrap()
            .contains("8.8.8.8".parse().unwrap()));
    }
    #[test]
    fn exact_address_prefixes() {
        let v4: Cidr = "192.0.2.1/32".parse().unwrap();
        assert!(v4.contains("192.0.2.1".parse().unwrap()));
        assert!(!v4.contains("192.0.2.2".parse().unwrap()));
        let v6: Cidr = "2001:db8::1/128".parse().unwrap();
        assert!(v6.contains("2001:db8::1".parse().unwrap()));
        assert!(!v6.contains("2001:db8::2".parse().unwrap()));
    }
    #[test]
    fn dns_is_never_implicit() {
        let p = Policy::parse("cidr 10.0.0.0/8 direct\nfinal block").unwrap();
        assert_eq!(p.evaluate(&flow("internal.example")).action, Action::Block);
        let f = Flow::new(
            "internal.example",
            Some("10.2.3.4".parse().unwrap()),
            443,
            Network::Tcp,
        )
        .unwrap();
        assert_eq!(p.evaluate(&f).action, Action::Direct);
    }
    #[test]
    fn rejects_ambiguous_and_malformed_policies() {
        for p in [
            "",
            "domain example.com direct",
            "final direct\nfinal block",
            "final block\nport 443 direct",
            "geoip CN direct\nfinal block",
            "port 0 direct\nfinal block",
            "cidr 10.0.0.0/33 direct\nfinal block",
            "network icmp direct\nfinal block",
            "final proxy:",
            "final typo",
            "domain *.example.com direct\nfinal block",
        ] {
            assert!(Policy::parse(p).is_err(), "accepted {p}");
        }
    }
    #[test]
    fn comments_preserve_line_numbers() {
        let p =
            Policy::parse("# comment\n\ndomain example.com direct # inline\nfinal block\n# end")
                .unwrap();
        assert_eq!(p.evaluate(&flow("example.com")).line, 3);
        assert_eq!(p.rule_count(), 1);
    }
    #[test]
    fn invalid_flows_fail() {
        for h in [
            "",
            "https://example.com",
            "example..com",
            "-example.com",
            "example.com/",
            "例子.cn",
        ] {
            assert!(Flow::new(h, None, 443, Network::Tcp).is_err());
        }
        assert!(Flow::new("example.com", None, 0, Network::Tcp).is_err());
        assert!(Flow::new(
            "192.0.2.1",
            Some("192.0.2.2".parse().unwrap()),
            443,
            Network::Tcp
        )
        .is_err());
    }
    #[test]
    fn port_and_protocol_are_distinct() {
        let p = Policy::parse("network udp block\nport 22 direct\nfinal proxy:default").unwrap();
        assert_eq!(
            p.evaluate(&Flow::new("example.com", None, 22, Network::Udp).unwrap())
                .action,
            Action::Block
        );
        assert_eq!(
            p.evaluate(&Flow::new("example.com", None, 22, Network::Tcp).unwrap())
                .action,
            Action::Direct
        );
        assert!(p.evaluate(&flow("example.com")).is_fallback);
    }
    #[test]
    fn pac_has_no_implicit_direct_fallback() {
        let p = Policy::parse("domain-suffix example.com direct\nfinal proxy:default").unwrap();
        let pac = p.to_pac("127.0.0.1:7890".parse().unwrap()).unwrap();
        assert!(pac.contains("dnsDomainIs(host, '.example.com')"));
        assert!(!pac.contains("; DIRECT"));
    }
    #[test]
    fn pac_rejects_lossy_rules_and_external_listener() {
        for p in [
            "final block",
            "cidr 10.0.0.0/8 direct\nfinal direct",
            "port 443 direct\nfinal direct",
            "final proxy:other",
        ] {
            assert!(Policy::parse(p)
                .unwrap()
                .to_pac("127.0.0.1:7890".parse().unwrap())
                .is_err());
        }
        assert!(Policy::parse("final direct")
            .unwrap()
            .to_pac("0.0.0.0:7890".parse().unwrap())
            .is_err());
    }
}

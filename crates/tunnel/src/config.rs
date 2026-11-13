use crate::{invalid, wire::Welcome, Result};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use subtle::ConstantTimeEq;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub endpoint: SocketAddr,
    pub server_name: String,
    pub ca_pem: String,
    pub device: String,
    pub token: String,
    pub ipv4: Ipv4Addr,
    pub ipv6: Ipv6Addr,
    pub routes: Vec<IpNet>,
    pub dns: Vec<IpAddr>,
}
impl Profile {
    pub fn validate(&self) -> Result<()> {
        if self.endpoint.port() == 0
            || self.endpoint.ip().is_unspecified()
            || self.endpoint.ip().is_multicast()
        {
            return Err(invalid("invalid server endpoint"));
        }
        if self.server_name.is_empty() || self.server_name.len() > 253 || self.ca_pem.len() > 32768
        {
            return Err(invalid("invalid TLS identity"));
        }
        let endpoint_in_tunnel = match self.endpoint.ip() {
            IpAddr::V4(ip) => ip.octets()[..3] == [10, 77, 0],
            IpAddr::V6(ip) => ip.segments()[..4] == [0xfd77, 0, 0, 0],
        };
        if endpoint_in_tunnel {
            return Err(invalid("server endpoint overlaps the tunnel subnet"));
        }
        validate_identity(&self.device, &self.token)?;
        validate_lease(self.ipv4, self.ipv6)?;
        if self.routes.is_empty()
            || self.routes.len() > 64
            || self.dns.is_empty()
            || self.dns.len() > 4
        {
            return Err(invalid("invalid routes or DNS"));
        }
        for ip in &self.dns {
            if *ip == self.endpoint.ip()
                || ip.is_unspecified()
                || ip.is_multicast()
                || !self.routes.iter().any(|r| r.contains(ip))
            {
                return Err(invalid("DNS must be inside a captured route"));
            }
        }
        Ok(())
    }
    pub fn welcome(&self) -> Welcome {
        Welcome {
            version: 1,
            ipv4: self.ipv4.into(),
            ipv6: self.ipv6.into(),
            mtu: 1280,
        }
    }
}
pub fn validate_lease(v4: Ipv4Addr, v6: Ipv6Addr) -> Result<()> {
    let o = v4.octets();
    let s = v6.segments();
    if o[..3] != [10, 77, 0]
        || o[3] < 2
        || o[3] > 254
        || s[..4] != [0xfd77, 0, 0, 0]
        || s[4..7] != [0, 0, 0]
        || s[7] < 2
    {
        return Err(invalid("lease must be 10.77.0.2-254 and fd77::2-ffff"));
    }
    Ok(())
}
pub fn validate_identity(device: &str, token: &str) -> Result<()> {
    if device.is_empty()
        || device.len() > 64
        || !device
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || token.len() != 64
        || !token
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(invalid("invalid device identity"));
    }
    Ok(())
}
pub fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    pub id: String,
    pub token_sha256: String,
    pub ipv4: Ipv4Addr,
    pub ipv6: Ipv6Addr,
}
impl Device {
    pub fn authenticates(&self, id: &str, token: &str) -> bool {
        validate_identity(id, token).is_ok()
            && self.id == id
            && bool::from(
                token_hash(token)
                    .as_bytes()
                    .ct_eq(self.token_sha256.as_bytes()),
            )
    }
    pub fn welcome(&self) -> Welcome {
        Welcome {
            version: 1,
            ipv4: self.ipv4.into(),
            ipv6: self.ipv6.into(),
            mtu: 1280,
        }
    }
    pub fn owns(&self, ip: IpAddr) -> bool {
        ip == IpAddr::V4(self.ipv4) || ip == IpAddr::V6(self.ipv6)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authentication_and_validation() {
        let token = "ab".repeat(32);
        let d = Device {
            id: "phone".into(),
            token_sha256: token_hash(&token),
            ipv4: "10.77.0.2".parse().unwrap(),
            ipv6: "fd77::2".parse().unwrap(),
        };
        assert!(d.authenticates("phone", &token));
        assert!(!d.authenticates("phone", &"cd".repeat(32)));
        assert!(!d.authenticates("other", &token));
        assert!(!d.authenticates("phone", ""));
        assert!(!d.owns("10.77.0.3".parse().unwrap()));
        assert!(validate_lease(d.ipv4, d.ipv6).is_ok());
        assert!(validate_lease("127.0.0.1".parse().unwrap(), d.ipv6).is_err());
    }
}

use crate::{invalid, wire::MTU, Result};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
/// Validate exact packet length before reading addresses. No Ethernet/PI header.
pub fn addresses(p: &[u8]) -> Result<(IpAddr, IpAddr)> {
    if p.is_empty() || p.len() > MTU {
        return Err(invalid("invalid packet length"));
    }
    match p[0] >> 4 {
        4 if p.len() >= 20 => {
            let header = (p[0] as usize & 15) * 4;
            let length = u16::from_be_bytes([p[2], p[3]]) as usize;
            if header < 20 || header > p.len() || length != p.len() {
                return Err(invalid("invalid IPv4 packet"));
            }
            Ok((
                Ipv4Addr::new(p[12], p[13], p[14], p[15]).into(),
                Ipv4Addr::new(p[16], p[17], p[18], p[19]).into(),
            ))
        }
        6 if p.len() >= 40 => {
            if u16::from_be_bytes([p[4], p[5]]) as usize + 40 != p.len() {
                return Err(invalid("invalid IPv6 packet"));
            }
            Ok((
                Ipv6Addr::from(<[u8; 16]>::try_from(&p[8..24]).unwrap()).into(),
                Ipv6Addr::from(<[u8; 16]>::try_from(&p[24..40]).unwrap()).into(),
            ))
        }
        _ => Err(invalid("invalid IP version")),
    }
}
/// Default public egress policy. An explicit server allow-list can admit private routes.
pub fn public_destination(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => {
            let o = a.octets();
            !(o[0] == 0
                || o[0] == 10
                || o[0] == 127
                || o[0] >= 224
                || (o[0] == 100 && (64..=127).contains(&o[1]))
                || (o[0] == 169 && o[1] == 254)
                || (o[0] == 172 && (16..=31).contains(&o[1]))
                || (o[0] == 192 && (o[1] == 168 || o[1] == 0))
                || (o[0] == 198 && (o[1] == 18 || o[1] == 19)))
        }
        IpAddr::V6(a) => {
            // Global unicast only; exclude mapped IPv4, transition and documentation ranges.
            let s = a.segments();
            s[0] & 0xe000 == 0x2000
                && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
                && s[0] != 0x2002
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packet_lengths_and_spoof_fields() {
        let mut v4 = vec![0u8; 20];
        v4[0] = 0x45;
        v4[3] = 20;
        v4[12..16].copy_from_slice(&[10, 77, 0, 2]);
        v4[16..20].copy_from_slice(&[1, 1, 1, 1]);
        assert_eq!(
            addresses(&v4).unwrap().0,
            "10.77.0.2".parse::<IpAddr>().unwrap()
        );
        v4[3] = 21;
        assert!(addresses(&v4).is_err());
        v4[3] = 20;
        v4[0] = 0x4f;
        assert!(addresses(&v4).is_err());
        let mut v6 = vec![0; 40];
        v6[0] = 0x60;
        assert!(addresses(&v6).is_ok());
        v6[5] = 1;
        assert!(addresses(&v6).is_err());
    }
    #[test]
    fn internal_egress_is_opt_in() {
        for ip in [
            "127.0.0.1",
            "169.254.169.254",
            "10.0.0.1",
            "100.64.0.1",
            "::1",
            "fd77::1",
            "::ffff:127.0.0.1",
            "fe80::1",
            "2002:7f00:1::",
        ] {
            assert!(!public_destination(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["1.1.1.1", "2606:4700:4700::1111"] {
            assert!(public_destination(ip.parse().unwrap()));
        }
    }
}

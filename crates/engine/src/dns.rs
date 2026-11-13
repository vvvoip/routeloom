//! Session-stable synthetic DNS addresses preserve domain identity per flow.
use hickory_proto::{
    op::{Message, MessageType, ResponseCode},
    rr::{
        rdata::{A, AAAA},
        Name, RData, Record, RecordType,
    },
};
use routeloom_tunnel::{invalid, Result};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};
#[derive(Default)]
pub struct Names {
    journal: Option<crate::cache::Journal>,
    last: u16,
    names: HashMap<String, u16>,
    reverse: HashMap<u16, String>,
}
impl Names {
    pub fn open(
        directory: &std::path::Path,
        profile: &routeloom_tunnel::config::Profile,
    ) -> Result<Self> {
        let (journal, records, last) = crate::cache::Journal::open(directory, profile)?;
        let mut names = Self {
            journal: Some(journal),
            last,
            ..Default::default()
        };
        for (id, host) in records {
            if names.names.insert(host.clone(), id).is_some() {
                return Err(invalid("duplicate cached domain"));
            }
            names.reverse.insert(id, host);
        }
        Ok(names)
    }
    fn id(&mut self, name: &str) -> Result<u16> {
        if let Some(id) = self.names.get(name) {
            return Ok(*id);
        }
        if self.names.len() >= 4096 {
            return Err(invalid("DNS table full"));
        }
        let id = self
            .last
            .checked_add(1)
            .filter(|id| *id < 65535)
            .ok_or_else(|| invalid("DNS address pool exhausted"))?;
        if let Some(journal) = self.journal.as_mut() {
            journal.append(id, name)?;
        }
        self.last = id;
        self.names.insert(name.into(), id);
        self.reverse.insert(id, name.into());
        Ok(id)
    }
    pub fn host(&self, ip: IpAddr) -> Result<String> {
        let id = match ip {
            IpAddr::V4(v) if v.octets()[..2] == [198, 18] => Some((u32::from(v) & 65535) as u16),
            IpAddr::V6(v) if v.segments()[..7] == [0xfdfe, 0, 0, 0, 0, 0, 0] => {
                Some(v.segments()[7])
            }
            _ => None,
        };
        match id {
            Some(id) => self
                .reverse
                .get(&id)
                .cloned()
                .ok_or_else(|| invalid("unknown synthetic DNS address")),
            None => Ok(ip.to_string()),
        }
    }
    pub fn answer(&mut self, bytes: &[u8]) -> Result<Vec<u8>> {
        let query = Message::from_vec(bytes).map_err(|_| invalid("invalid DNS"))?;
        if query.message_type() != MessageType::Query || query.queries().len() != 1 {
            return Err(invalid("one DNS question required"));
        }
        let q = &query.queries()[0];
        let mut reply = Message::new();
        reply
            .set_id(query.id())
            .set_message_type(MessageType::Response)
            .set_recursion_desired(query.recursion_desired())
            .set_recursion_available(true)
            .add_query(q.clone());
        let host = q
            .name()
            .to_ascii()
            .trim_end_matches('.')
            .to_ascii_lowercase();
        if routeloom_core::Flow::new(&host, None, 53, routeloom_core::Network::Udp).is_err() {
            reply.set_response_code(ResponseCode::Refused);
        } else if matches!(q.query_type(), RecordType::A | RecordType::AAAA) {
            match self.id(&host) {
                Ok(id) => {
                    let data = if q.query_type() == RecordType::A {
                        RData::A(A(Ipv4Addr::from(0xc6120000u32 + u32::from(id))))
                    } else {
                        RData::AAAA(AAAA(Ipv6Addr::new(0xfdfe, 0, 0, 0, 0, 0, 0, id)))
                    };
                    reply.add_answer(Record::from_rdata(
                        Name::from_ascii(&host).map_err(|_| invalid("invalid DNS name"))?,
                        30,
                        data,
                    ));
                }
                Err(_) => {
                    reply.set_response_code(ResponseCode::ServFail);
                }
            }
        }
        // Unsupported types return NODATA: never leak an alternate real A/AAAA via HTTPS/SVCB.
        reply.to_vec().map_err(|_| invalid("DNS encoding failed"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use hickory_proto::op::Query;
    #[test]
    fn stable_names_and_unknown_addresses() {
        let mut names = Names::default();
        let mut q = Message::new();
        q.add_query(Query::query(
            Name::from_ascii("Example.com").unwrap(),
            RecordType::A,
        ));
        let a = names.answer(&q.to_vec().unwrap()).unwrap();
        let r = Message::from_vec(&a).unwrap();
        assert_eq!(r.answers().len(), 1);
        assert_eq!(
            names.host("198.18.0.1".parse().unwrap()).unwrap(),
            "example.com"
        );
        assert_eq!(
            names.host("fdfe::1".parse().unwrap()).unwrap(),
            "example.com"
        );
        assert!(names.host("198.18.0.2".parse().unwrap()).is_err());
        assert_eq!(names.host("1.1.1.1".parse().unwrap()).unwrap(), "1.1.1.1");
        assert_eq!(names.answer(&q.to_vec().unwrap()).unwrap(), a);
    }
}

//! mDNS resource record.

use crate::dns::{Answer, Packet, Question, SrvDataA, SrvDataQ};
use get_if_addrs;
use hostname;
use std::{collections::HashMap, net::Ipv4Addr};

/// Type helper corresponding to the resource record of a host.
pub type MapRecord = HashMap<String, Record>;

#[derive(Debug, Clone)]
pub struct Record {
    /// Hostname of our neighbor
    pub(crate) hostname: String,
    /// IP address in the lan
    pub(crate) addrs: Vec<Ipv4Addr>,
    /// Time to live
    pub(crate) ttl: u32,
    // /// When this resource was recorded
    // timestamp: Option<>
}

impl Record {
    /// Create a new record respecting the mDNS
    /// [naming convention](https://tools.ietf.org/html/rfc6762#section-3)
    /// of the form "single-dns-label.local." with value ending with `.local.`
    pub fn new(name: &str, addr: &[Ipv4Addr]) -> Self {
        let hostname = convert_to_mdns_hostname(&name);

        Record {
            hostname,
            addrs: addr.to_vec(),
            ttl: 60,
        }
    }

    /// Returns a reference to the IP address of a neighbor in the LAN.
    pub fn addr(&self) -> &[Ipv4Addr] {
        &self.addrs
    }

    /// Returns a reference to the hostname of a neighbor in the LAN.
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Returns the time to leave value oif a [`Record`].
    pub fn ttl(&self) -> u32 {
        self.ttl
    }

    /// Build a host own record. If there we fail to gather IPv4 addresses from the system,
    /// we fall back to "0.0.0.0" address.
    pub fn new_own() -> Self {
        let hostname = convert_to_mdns_hostname(
            &hostname::get_hostname().unwrap_or(String::from("Anonymous-host")),
        );

        let mut addrs: Vec<Ipv4Addr> = get_if_addrs::get_if_addrs()
            .expect("Fail to retrieve host network interfaces.")
            .iter()
            .filter_map(|iface| {
                if !iface.is_loopback() {
                    match &iface.addr {
                        get_if_addrs::IfAddr::V4(ipv4) => Some(ipv4.ip),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect();

        if addrs.is_empty() {
            addrs = vec!["0.0.0.0"
                .parse()
                .expect("Fail to parse default IPv4 address.")]
        }

        let ttl = 120;

        Record { hostname, addrs, ttl }
    }

    /// Convert a [`Packet`](crate::dns::Packet) to a [`Record`].
    pub fn from_packet(packet: &Packet) -> MapRecord {
        let records: Vec<Record> = packet
            .answers
            .iter()
            .filter_map(|answer| match answer {
                Answer::Unknown(_) => None,
                Answer::Srv(sda) => {
                    let hostname =
                        std::str::from_utf8(&sda.name).expect("Fail to convert bytes to hostname.");
                    let addr: String = std::str::from_utf8(&sda.target)
                        .expect("Fail to convert bytes to IP address.")
                        .to_string();
                    let addr: Ipv4Addr = addr
                        .parse()
                        .expect("Fail to parse String IP address to Ipv4Addr.");
                    Some(Record::new(&hostname, &[addr]))
                }
            })
            .collect();

        dbg!(&records);

        let mut map_record = MapRecord::with_capacity(records.len());
        for new_record in records.iter() {
            if let Some(rec) = map_record.get(&new_record.hostname) {
                let new_addr = new_record.addrs.first().expect("Empty list of address.");
                let mut record_to_update = rec.clone();

                if !record_to_update.addrs.contains(new_addr) {
                    record_to_update.addrs.push(*new_addr);
                }
                map_record.insert(new_record.hostname.clone(), record_to_update.clone());
            } else {
                map_record.insert(new_record.hostname.clone(), new_record.clone());
            }
        }
        map_record
    }

    /// Convert multiple [`Record`] to a vector of [`Answer Response`](crate::dns::Answer).
    pub fn to_vector_answers(&self) -> Vec<Answer> {
        let mut answers = Vec::with_capacity(self.addrs.len());
        for addr in &self.addrs {
            answers.push(Answer::Srv(SrvDataA {
                name: self.hostname.as_bytes().to_vec(),
                ttl_seconds: 255,
                priority: 0,
                weight: 0,
                port: crate::SERVICE_LISTENER_PORT,
                target: addr.to_string().as_bytes().to_vec(),
            }))
        }

        answers
    }

    /// Convert a [`Record`] to an [`Answer Reponse`](crate::dns::Answer).
    pub fn to_answers_response_packet(&self) -> Packet {
        Packet {
            id: 0x0,
            is_query: false,
            questions: vec![],
            answers: self.to_vector_answers(),
        }
    }

    /// Convert a record to a question packet.
    pub fn to_query_question_packet(&self) -> Packet {
        Packet {
            id: 0x0,
            is_query: true,
            questions: vec![Question::Srv(SrvDataQ {
                name: self.hostname().as_bytes().to_vec(),
            })],
            answers: vec![],
        }
    }
}

fn convert_to_mdns_hostname(hostname: &str) -> String {
    if hostname.ends_with(".local.") {
        hostname.to_string()
    } else {
        format!("{}.local.", hostname)
    }
}

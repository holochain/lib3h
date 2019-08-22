//! mDNS resource record definition.

use crate::dns::{AnswerSection, DnsMessage, QuerySection, Target};
use get_if_addrs;
use hostname;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

/// Helper type.
pub type HashMapRecord = HashMap<String, Record>;

/// Type helper corresponding to the resource record of a host.
#[derive(Debug, Clone)]
pub struct MapRecord {
    pub(crate) value: HashMapRecord,
}

impl Deref for MapRecord {
    type Target = HashMapRecord;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for MapRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl MapRecord {
    pub fn new(hostname: &str, record: &Record) -> Self {
        let mut hmr = HashMapRecord::new();
        hmr.insert(hostname.to_string(), record.clone());

        Self { value: hmr }
    }

    /// Update the [`MapRecord`]'s vectors of addresses.
    pub fn update(&mut self, new_map_record: &MapRecord) {
        for (_name, new_record) in new_map_record.iter() {
            if let Some(rec) = self.value.get(&new_record.hostname) {
                let new_addr = new_record.addrs.first().expect("Empty list of address.");
                let mut record_to_update = rec.clone();

                if !record_to_update.addrs.contains(new_addr) {
                    record_to_update.addrs.push(new_addr.clone());
                }

                self.value
                    .insert(new_record.hostname.clone(), record_to_update.clone());
            } else {
                self.value
                    .insert(new_record.hostname.clone(), new_record.clone());
            }
        }
    }

    /// Builds a [`MapRecord`] from a [`DnsMessage`].
    pub fn from_dns_message(dmesg: &DnsMessage) -> Option<MapRecord> {
        if !dmesg.answers.is_empty() {
            let records: Vec<Record> = dmesg
                .answers
                .iter()
                .map(|a_sec| {
                    let targets: Vec<String> =
                        a_sec.data.iter().map(|t| t.target.clone()).collect();
                    Record::new(&a_sec.domain_name, &targets, a_sec.ttl)
                })
                .collect();

            let mut map_record = MapRecord {
                value: HashMapRecord::with_capacity(records.len()),
            };
            for new_record in records.iter() {
                if let Some(rec) = map_record.get(&new_record.hostname) {
                    let new_addr = new_record.addrs.first().expect("Empty list of address.");
                    let mut record_to_update = rec.clone();

                    if !record_to_update.addrs.contains(new_addr) {
                        record_to_update.addrs.push(new_addr.clone());
                    }
                    map_record.insert(new_record.hostname.clone(), record_to_update.clone());
                } else {
                    map_record.insert(new_record.hostname.clone(), new_record.clone());
                }
            }

            Some(map_record)
        } else {
            None
        }
    }

    /// Builds a [`DnsMessage`] from a [`MapRecord`].
    pub fn to_dns_reponse_message(&self) -> DnsMessage {
        let answers: Vec<AnswerSection> = self
            .value
            .values()
            .map(|record| record.to_answers_section())
            .collect();

        DnsMessage {
            nb_answers: answers.len() as u16,
            answers,
            ..DnsMessage::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// Hostname of our neighbor
    pub(crate) hostname: String,
    /// IP address in the lan
    pub(crate) addrs: Vec<String>,
    /// Time to live
    pub(crate) ttl: u32,
}

impl Record {
    /// Create a new record respecting the mDNS
    /// [naming convention](https://tools.ietf.org/html/rfc6762#section-3)
    /// of the form "single-dns-label.local." with value ending with `.local.`
    pub fn new(name: &str, addr: &[String], ttl: u32) -> Self {
        let hostname = convert_to_mdns_hostname(&name);

        Record {
            hostname,
            addrs: addr.to_vec(),
            ttl,
        }
    }

    /// Returns a reference to the IP address of a neighbor in the LAN.
    pub fn addr(&self) -> &[String] {
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
            &hostname::get_hostname().unwrap_or_else(|| String::from("Anonymous-host")),
        );

        let mut addrs: Vec<String> = get_if_addrs::get_if_addrs()
            .expect("Fail to retrieve host network interfaces.")
            .iter()
            .filter_map(|iface| {
                if !iface.is_loopback() {
                    match &iface.addr {
                        get_if_addrs::IfAddr::V4(ipv4) => Some(ipv4.ip.to_string()),
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

        Record {
            hostname,
            addrs,
            ttl: 255,
        }
    }

    // /// Convert multiple [`Record`] to a vector of [`Answer Response`](crate::dns::Answer).
    // pub fn to_vector_answers(&self) -> Vec<Answer> {
    //     let mut answers = Vec::with_capacity(self.addrs.len());
    //     for addr in &self.addrs {
    //         answers.push(Answer::Srv(SrvDataA {
    //             name: self.hostname.as_bytes().to_vec(),
    //             ttl_seconds: 255,
    //             priority: 0,
    //             weight: 0,
    //             port: crate::SERVICE_LISTENER_PORT,
    //             target: addr.to_string().as_bytes().to_vec(),
    //         }))
    //     }
    //
    //     answers
    // }

    /// Convert a [`Record`] to an [`Answer Reponse`](crate::dns::Answer).
    pub fn to_answers_section(&self) -> AnswerSection {
        AnswerSection::new_with_ttl(&self.hostname, &self.to_targets(), self.ttl)
    }

    /// Convert a record to a question packet.
    pub fn to_query_question(&self) -> QuerySection {
        QuerySection::new(&self.hostname)
    }

    pub fn to_targets(&self) -> Vec<Target> {
        self.addrs.iter().map(|addr| Target::new(addr)).collect()
    }
}

fn convert_to_mdns_hostname(hostname: &str) -> String {
    if hostname.ends_with(".local.") {
        hostname.to_string()
    } else {
        format!("{}.local.", hostname)
    }
}

// /// Convert a [`Packet`](crate::dns::Packet) to a [`Record`].
// pub fn from_packet(packet: &Packet) -> Option<MapRecord> {
//     let records: Vec<Record> = packet
//         .answers
//         .iter()
//         .filter_map(|answer| match answer {
//             Answer::Unknown(_) => None,
//             Answer::Srv(sda) => {
//                 let hostname =
//                     std::str::from_utf8(&sda.name).expect("Fail to convert bytes to hostname.");
//                 let addr: String = std::str::from_utf8(&sda.target)
//                     .expect("Fail to convert bytes to IP address.")
//                     .to_string();
//                 let addr: Ipv4Addr = addr
//                     .parse()
//                     .expect("Fail to parse String IP address to Ipv4Addr.");
//                 Some(Record::new(&hostname, &[addr]))
//             }
//         })
//     .collect();
//
//     if records.len() > 0 {
//
//         let mut map_record = MapRecord::with_capacity(records.len());
//         for new_record in records.iter() {
//             if let Some(rec) = map_record.get(&new_record.hostname) {
//                 let new_addr = new_record.addrs.first().expect("Empty list of address.");
//                 let mut record_to_update = rec.clone();
//
//                 if !record_to_update.addrs.contains(new_addr) {
//                     record_to_update.addrs.push(*new_addr);
//                 }
//                 map_record.insert(new_record.hostname.clone(), record_to_update.clone());
//             } else {
//                 map_record.insert(new_record.hostname.clone(), new_record.clone());
//             }
//         }
//
//         Some(map_record)
//     } else {
//         None
//     }
// }

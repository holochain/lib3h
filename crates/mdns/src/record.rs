//! mDNS resource record definition.

use crate::{
    dns::{AnswerSection, DnsMessage, QuerySection, Target},
    DEFAULT_BIND_ADRESS, DEFAULT_TTL,
};
use hostname;
use std::{
    cmp::Ordering,
    collections::HashMap,
    ops::{Deref, DerefMut},
};

/// Helper type corresponding to a HashMap<NetworkId, Vec<Url>>
pub type HashMapRecord = HashMap<String, Vec<Record>>;

/// Type helper corresponding to the resource record of a host.
#[derive(Debug, Clone)]
pub struct MapRecord(pub(crate) HashMapRecord);

impl Deref for MapRecord {
    type Target = HashMapRecord;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MapRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl MapRecord {
    pub fn new(networkid: &str, record: &[Record]) -> Self {
        let mut hmr = HashMapRecord::new();
        hmr.insert(networkid.to_string(), record.to_vec());

        Self(hmr)
    }

    /// Update the [`MapRecord`]'s vectors of addresses.
    pub fn update(&mut self, other_map_record: &MapRecord) {
        for (other_nid, other_records) in other_map_record.iter() {
            // Update an existing one
            if let Some(cached_records) = self.0.get_mut(other_nid) {
                for other_rec in other_records.iter() {
                    cached_records.push(other_rec.clone());
                }
                cached_records.sort();
                cached_records.dedup_by(|a, b| a.url() == b.url());
            } else {
                self.0.insert(other_nid.to_string(), other_records.to_vec());
            }
        }
    }

    /// Builds a [`MapRecord`] from a [`DnsMessage`].
    pub fn from_dns_message(dmesg: &DnsMessage) -> Option<MapRecord> {
        if !dmesg.answers.is_empty() {
            let mut records: Vec<Record> = dmesg
                .answers
                .iter()
                .map(|a_sec| Record::new(&a_sec.domain_name, &a_sec.data.target, a_sec.ttl))
                .collect();

            records.sort();
            records.dedup_by(|a, b| a.url() == b.url());

            let mut map_record = MapRecord(HashMapRecord::with_capacity(records.len()));
            for new_record in records.iter() {
                let fake_map_record = MapRecord::new(&new_record.networkid, &[new_record.clone()]);
                map_record.update(&fake_map_record);
            }

            Some(map_record)
        } else {
            None
        }
    }

    /// Builds a [`DnsMessage`] from a [`MapRecord`].
    pub fn to_dns_response_message(&self, networkids: &[&str]) -> Option<DnsMessage> {
        // Let's make sure we have at least one networkid in our keys
        let mut answers = Vec::new();

        for &netid in networkids.iter() {
            if let Some(records) = self.0.get(netid) {
                for rec in records {
                    answers.push(rec.to_answers_section());
                }
            }
        }

        if answers.is_empty() {
            None
        } else {
            Some(DnsMessage {
                nb_answers: answers.len() as u16,
                nb_authority: answers.len() as u16,
                answers,
                ..DnsMessage::default()
            })
        }
    }

    /// Builds a [`DnsMessage`] from a [`MapRecord`].
    pub fn to_dns_message_query(&self, networkids: &[&str]) -> Option<DnsMessage> {
        // Let's make sure we have at least one networkid in our keys
        let mut questions = Vec::new();

        for &netid in networkids.iter() {
            if let Some(records) = self.0.get(netid) {
                for rec in records {
                    questions.push(rec.to_question_section());
                }
            }
        }

        if questions.is_empty() {
            None
        } else {
            Some(DnsMessage {
                nb_questions: questions.len() as u16,
                questions,
                ..DnsMessage::default()
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// Hostname of our neighbor
    pub(crate) networkid: String,
    /// IP address in the lan
    pub(crate) url: String,
    /// Time to live
    pub(crate) ttl: u32,
}

impl Record {
    /// Create a new record respecting the mDNS
    /// [naming convention](https://tools.ietf.org/html/rfc6762#section-3)
    /// of the form "single-dns-label.local." with value ending with `.local.`
    pub fn new(name: &str, url: &str, ttl: u32) -> Self {
        Record {
            networkid: name.to_owned(),
            url: url.to_owned(),
            ttl,
        }
    }

    /// Returns a reference to the IP address of a neighbor in the LAN.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns a reference to the hostname of a neighbor in the LAN.
    pub fn networkid(&self) -> &str {
        &self.networkid
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

        Record {
            networkid: hostname,
            url: String::from(DEFAULT_BIND_ADRESS),
            ttl: DEFAULT_TTL,
        }
    }

    /// Convert a [`Record`] to an [`Answer Reponse`](crate::dns::AnswerSection).
    pub fn to_answers_section(&self) -> AnswerSection {
        AnswerSection::new_with_ttl(&self.networkid, &self.to_target(), self.ttl)
    }

    /// Convert a record to a question packet.
    pub fn to_question_section(&self) -> QuerySection {
        QuerySection::new(&self.networkid)
    }

    pub fn to_target(&self) -> Target {
        Target::new(&self.url)
    }

    /// Builds a [`DnsMessage`] from a [`MapRecord`].
    pub fn to_dns_response_message(&self) -> DnsMessage {
        DnsMessage {
            nb_answers: 1,
            nb_authority: 1,
            answers: vec![self.to_answers_section()],
            ..DnsMessage::default()
        }
    }
}

impl Ord for Record {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.networkid.cmp(&other.networkid) {
            Ordering::Equal => self.ttl.cmp(&other.ttl),
            other_ordering => other_ordering,
        }
    }
}

impl PartialOrd for Record {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// This function is usefull if we want to be fully mDNS compliant, but it's not the case at the
/// moment, so it's not used in our current implementaition.
fn convert_to_mdns_hostname(hostname: &str) -> String {
    if hostname.ends_with(".local.") {
        hostname.to_string()
    } else {
        format!("{}.local.", hostname)
    }
}

#[test]
fn map_record_update_test() {
    let networkid = "hcnmynetworkid.hc-mdns-discovery.holo.host";
    let url = "wss://1.2.3.4:12345?a=HcMmymachineid";
    let record_to_prune1 = Record::new(networkid, url, 255);
    let record_to_prune2 = Record::new(networkid, url, 200);
    // Because this record has the smallest ttl, it's the one that supposed to be kept during the dedup
    // process
    let record_to_keep = Record::new(networkid, url, 100);
    let mut map_record = MapRecord::new(
        networkid,
        &[record_to_prune1, record_to_prune2, record_to_keep.clone()],
    );

    for (_, records) in map_record.iter_mut() {
        records.sort();
        records.dedup_by(|a, b| a.url() == b.url());
    }

    if let Some(dedup_records) = map_record.get(networkid) {
        assert_eq!(dedup_records, &[record_to_keep])
    }
}

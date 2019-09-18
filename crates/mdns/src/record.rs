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
    pub fn new() -> Self {
        Self(HashMapRecord::new())
    }

    /// Creates a new [`MapRecord`] with one record.
    pub fn with_record(networkid: &str, record: &[Record]) -> Self {
        let mut hmr = HashMapRecord::new();
        hmr.insert(networkid.to_string(), record.to_vec());

        Self(hmr)
    }

    /// Update the [`MapRecord`]'s vectors of addresses.
    pub fn update(&mut self, other_map_record: &MapRecord) {
        // TODO: find a better way to do it.
        for (other_nid, other_records) in other_map_record.iter() {
            if let Some(cached_records) = self.0.get_mut(other_nid) {
                for other_rec in other_records.iter() {
                    cached_records.push(other_rec.clone());
                }

                cached_records.sort();

                let mut uniq_urls: Vec<String> =
                    cached_records.iter().map(|rec| {
                        rec.url.clone()
                    }).collect();
                // let mut uniq_urls: Vec<String> =
                //     cached_records.iter().filter_map(|rec| {
                //         // Let's not add dead record
                //         if rec.ttl > 0 {
                //             Some(rec.url.clone())
                //         } else {
                //             None
                //         }
                //     }).collect();

                uniq_urls.dedup();

                let mut final_records: Vec<Record> = Vec::new();

                for url in uniq_urls {
                    let drain_record_with_url: Vec<Record> = cached_records
                        .drain_filter(|rec| *rec.url == *url)
                        .collect();
                    if drain_record_with_url.len() == 1 {
                        final_records.push(drain_record_with_url[0].clone());
                    } else {
                        if let Some(first_record) = drain_record_with_url.first() {
                            if first_record.ttl == 0 {
                                final_records.push(first_record.clone());
                            }
                        }
                        if let Some(last_record) = drain_record_with_url.last() {
                            if last_record.ttl == 255 {
                                final_records.push(last_record.clone());
                            }
                        }
                    }
                }

                *cached_records = final_records.clone();
            } else {
                self.0.insert(other_nid.to_string(), other_records.to_vec());
            }
        }
    }

    /// Builds a [`MapRecord`] from a [`DnsMessage`].
    pub fn from_dns_message(dmesg: &DnsMessage) -> Option<MapRecord> {
        if !dmesg.answers.is_empty() {
            // Let's create a vector of records, applying a filter on the class and type
            // (INET + CNAME)
            let mut records: Vec<Record> = dmesg
                .answers
                .iter()
                .filter_map(|a_sec| {
                    if a_sec.answer_class == 1 && a_sec.answer_type == 5 {
                        Some(Record::new(
                            &a_sec.domain_name,
                            &a_sec.data.target,
                            a_sec.ttl,
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            records.sort();
            records.dedup_by(|a, b| a.url() == b.url());

            let mut map_record = MapRecord(HashMapRecord::with_capacity(records.len()));
            for new_record in records.iter() {
                // Create a temporary MapRecord so we can update the one that we are about to
                // return with the new records we found on the network
                let tmp_map_record =
                    MapRecord::with_record(&new_record.networkid, &[new_record.clone()]);
                map_record.update(&tmp_map_record);
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
            Ordering::Equal => match self.url.cmp(&other.url) {
                Ordering::Equal => self.ttl.cmp(&other.ttl),
                other_ordering => other_ordering,
            },
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
    let mut map_record = MapRecord::with_record(
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

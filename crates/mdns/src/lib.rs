//! lib3h mDNS LAN discovery module
//!
//! Our simple use case is the following:
//! ```rust
//! use lib3h_mdns as mdns;
//! use lib3h_discovery::Discovery;
//! use std::{thread, time::Duration};
//!
//! let mut mdns = mdns::MulticastDnsBuilder::new()
//!     // Let's define our own networkId (the network we operate on) and how to access us
//!     .own_record("holonaute.holo.host", &["wss://192.168.0.87:88088?a=hc0"])
//!     // Sets the interval between two automatic queries
//!     .query_interval_ms(1_000)
//!     .bind_port(8585)
//!     .build()
//!     .expect("Fail to build mDNS.");
//!
//! // Make myself known on the network and find a name for myself
//! mdns.advertise()
//!     .expect("Fail to advertise my existence to the world.");
//!
//! // Let's listen to the network for a few moments...
//! for _ in 0..5 {
//!     mdns.discover();
//!     println!("mDNS neighbourhood : {:#?}", &mdns.records());
//!
//!     thread::sleep(Duration::from_millis(100));
//! }
//! ```

#![feature(try_trait)]
#![feature(never_type)]
#![feature(drain_filter)]

extern crate lib3h_protocol;

use log::{debug, error, trace};
// Used to clean our buffer to avoid mixing messages together.
use url::Url;
use zeroize::Zeroize;

use std::{
    net::{self, SocketAddr, ToSocketAddrs},
    time::Instant,
};

use lib3h_discovery::{error::DiscoveryResult, Discovery};
use lib3h_protocol::uri::Lib3hUri;

pub mod error;
pub use error::{MulticastDnsError, MulticastDnsResult};

pub mod dns;
pub use dns::*;

pub mod builder;
pub use builder::MulticastDnsBuilder;

pub mod record;
use record::{MapRecord, Record};

// 20 byte IP header would mean 65_507... but funky configs can increase that
// const READ_BUF_SIZE: usize = 60_000;
// however... we don't want to accept any packets that big...
// let's stick with one common block size
const READ_BUF_SIZE: usize = 4_096;

/// Delay between probe query, 250ms by default.
const _PROBE_QUERY_DELAY_MS: u64 = 250;

/// Listening port of this mDNS service.
const SERVICE_LISTENER_PORT: u16 = 8585;

/// Threshold value used to getting ourselves out of a potential
/// infinite loop during probe
const _FAIL_SAFE_TRESHOLD: u16 = 1_000;

/// mDNS multicast IPv4 address.
const MDNS_MULCAST_IPV4_ADRESS: &str = "224.0.0.251";

/// Default bind adress.
const DEFAULT_BIND_ADRESS: &str = "0.0.0.0";

/// Default "time to live" value for a new record.
const DEFAULT_TTL: u32 = 255;

/// Default amount of time between two queries.
const DEFAULT_QUERY_INTERVAL_MS: u128 = 30_000;

/// an mdns instance that can send and receive dns packets on LAN UDP multicast
pub struct MulticastDns {
    /// Our IP address bound to UDP Socket, default to `0.0.0.0`
    pub(crate) bind_address: String,
    /// Port used by the mDNS protocol. mDNS use the `5353` by default
    pub(crate) bind_port: u16,
    /// If true, multicast packets will be looped back to the local socket
    pub(crate) multicast_loop: bool,
    /// Time to Live: default to `255`
    pub(crate) multicast_ttl: u32,
    /// Multicast address used by the mDNS protocol: `224.0.0.251`
    pub(crate) multicast_address: String,
    /// Determine if we need to query / announce.
    pub(crate) timestamp: Instant,
    /// The amount of time we should wait between two queries.
    pub(crate) query_interval_ms: u128,
    /// The socket used by the mDNS service protocol to send packets
    pub(crate) send_socket: net::UdpSocket,
    /// The socket used to receive mDNS packets
    pub(crate) recv_socket: net::UdpSocket,
    /// The buffer used to store the packet to send/receive messages
    buffer: [u8; READ_BUF_SIZE],
    /// Reference the host's record
    pub(crate) own_map_record: MapRecord,
    /// The lookup table where the neighbors are stored
    pub(crate) map_record: MapRecord,
}

impl MulticastDns {
    /// IP address of the mDNS server.
    pub fn address(&self) -> &str {
        &self.bind_address
    }

    /// the mDNS service port on the machine.
    pub fn port(&self) -> u16 {
        self.bind_port
    }

    /// Returns wether multicasting is set to loop or not.
    pub fn multicast_loop(&self) -> bool {
        self.multicast_loop
    }

    /// Returns the time to live value.
    pub fn multicast_ttl(&self) -> u32 {
        self.multicast_ttl
    }

    /// Returns the multicast address used by mDNS
    pub fn multicast_address(&self) -> &str {
        &self.multicast_address
    }

    /// Returns the lookup table of records as a [HashMap](std::collections::HashMap).
    pub fn records(&self) -> &MapRecord {
        &self.map_record
    }

    /// Returns a vector of all the url we own through every NetworkId.
    pub fn own_urls(&self) -> Vec<String> {
        self.own_map_record
            .iter()
            .flat_map(|(_, v)| v.iter().map(|r| r.url.clone()).collect::<Vec<String>>())
            .collect()
    }

    /// Returns all the urls for every NetworkId.
    pub fn urls(&self) -> Vec<Url> {
        self.map_record
            .iter()
            .flat_map(|(_, v)| {
                v.iter()
                    .filter_map(|r| match Url::parse(&r.url) {
                        Ok(url) => Some(url),
                        Err(_) => None,
                    })
                    .collect::<Vec<Url>>()
            })
            .collect()
    }

    /// Returns all the NetworkIds we are currently in.
    pub fn own_networkids(&self) -> Vec<&str> {
        self.own_map_record.keys().map(|k| k.as_str()).collect()
    }

    /// Returns the amount of time we wait between two queries.
    pub fn query_interval_ms(&self) -> u128 {
        self.query_interval_ms
    }

    /// Insert a new record to our cache.
    pub fn insert_record(&mut self, hostname: &str, records: &[Record]) {
        self.map_record
            .insert(hostname.to_string(), records.to_vec());
    }

    /// Update our cache of resource records.
    pub fn update_cache(&mut self, other_map_record: &MapRecord) {
        self.map_record.update(other_map_record);
    }

    /// Broadcasts a DNS message.
    pub fn broadcast_message(&self, dmesg: &DnsMessage) -> Result<usize, MulticastDnsError> {
        let addr = (self.multicast_address.as_ref(), self.bind_port)
            .to_socket_addrs()?
            .next()?;
        let data = dmesg.to_raw()?;

        Ok(self.send_socket.send_to(&data, &addr)?)
    }

    /// Broadcasts a packet.
    pub fn broadcast(&self, data: &[u8]) -> Result<usize, MulticastDnsError> {
        let addr = (self.multicast_address.as_ref(), self.bind_port)
            .to_socket_addrs()?
            .next()?;

        Ok(self.send_socket.send_to(&data, &addr)?)
    }

    /// try to receive a dns packet.
    /// will return None rather than blocking if none are queued
    pub fn recv(&mut self) -> MulticastDnsResult<Option<(Vec<u8>, SocketAddr)>> {
        self.clear_buffer();

        match self.recv_socket.recv_from(&mut self.buffer) {
            Ok((0, _)) => Ok(None),
            Ok((num_bytes, addr)) => {
                debug!(
                    "Received '{}' bytes: {:?}",
                    num_bytes,
                    &self.buffer.to_vec()[..num_bytes]
                );
                let packet = self.buffer[..num_bytes].to_vec();
                Ok(Some((packet, addr)))
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    Ok(None)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    /// Clean our buffer of bytes from previous messages.
    pub fn clear_buffer(&mut self) {
        self.buffer.zeroize();
    }

    /// Clean our cache by removing the out of live records.
    pub fn prune_cache(&mut self) {
        // Get the entry of the dead records to remove them safely afterward
        // let mut dead_entry_list: Vec<String> = Vec::with_capacity(self.map_record.len());
        for (_, records) in self.map_record.iter_mut() {
            let _: Vec<Record> = records.drain_filter(|r| r.ttl == 0).collect();
        }
    }

    /// Update the `time to live` of every cached record.
    pub fn update_ttl(&mut self) {
        let own_urls = self.own_urls();
        for (_netid, records) in self.map_record.iter_mut() {
            for record in records {
                if !own_urls.contains(&record.url) {
                    if record.ttl > 0 {
                        record.ttl -= 1;
                    }
                }
            }
        }
    }

    /// mDNS Querier, implementing the "One-Shot Multicast DNS Queries" from the standard.
    pub fn query(&mut self) -> MulticastDnsResult<()> {
        if let Some(query_message) = self.build_query_message() {
            self.broadcast_message(&query_message)?;
            self.broadcast_message(&query_message)?;
            self.broadcast_message(&query_message)?;
        }

        Ok(())
    }

    /// A mDNS Responder that listen to the network in order to repond to the queries.
    fn responder(&mut self) -> MulticastDnsResult<()> {
        // Process all elements of the UDP socket stack
        loop {
            match self.recv() {
                Ok(Some((packet, sender_addr))) => {
                    let dmesg = DnsMessage::from_raw(&packet)?;

                    // Here we update our cache with the responses gathered from the network
                    if dmesg.nb_answers > 0 {
                        if let Some(new_map_record) = MapRecord::from_dns_message(&dmesg) {
                            let own_networkids: Vec<String> = self
                                .own_networkids()
                                .iter()
                                .map(|v| v.to_string())
                                .collect();

                            // Only update our cache with the answers for our own NetworkId
                            for (netid, new_records) in new_map_record.iter() {
                                // Let's only operate on the networks we belong to
                                if own_networkids.contains(netid) {
                                    let tmp_new_map_record =
                                        MapRecord::with_record(netid, new_records);
                                    self.update_cache(&tmp_new_map_record);
                                }
                            }
                        }
                    }
                    // According to the standard: "Multicast DNS responses MUST NOT contain
                    // any questions in the Question Section.  Any questions in the
                    // Question Section of a received Multicast DNS response MUST be silently ignored

                    // We send response only for record we have authority on.
                    // We send the response directly to the sender instead of broadcasting it to
                    // avoid any unnecessary burden on the network.
                    else if dmesg.nb_questions > 0 {
                        let question_list: Vec<&str> = dmesg
                            .questions
                            .iter()
                            .filter_map(|q| {
                                // Filter out all the queries that are not INET + CNAME
                                if q.query_class == 1 && q.query_type == 5 {
                                    Some(q.domain_name.as_str())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if let Some(response) =
                            self.own_map_record.to_dns_response_message(&question_list)
                        {
                            self.send_socket.send_to(&response.to_raw()?, sender_addr)?;
                            // As the direct send message to the querier tends to fail on a local
                            // machine during our tests, we broadcast the response as well for
                            // safety reasons
                            self.broadcast_message(&response)?;
                        }
                    }
                }
                Ok(None) => {
                    trace!(">> Nothing on the UDP stack");
                    break;
                }
                Err(e) => {
                    error!(
                        "Something went wrong while processing the UDP stack during update: '{}'",
                        e
                    );
                    break;
                }
            }
        }
        Ok(())
    }

    /// Builds mDNS probe packet with the proper bit set up in order to check if a host record is
    /// available.
    /// Not used çin our implementation because we don't need to be fully standart compliant.
    fn _build_probe_packet(&self) -> DnsMessage {
        let questions: Vec<QuerySection> = self
            .own_map_record
            .keys()
            .map(|k| QuerySection::new(k))
            .collect();
        DnsMessage {
            nb_questions: questions.len() as u16,
            questions,
            ..Default::default()
        }
    }

    /// Builds a query DNS message to be used by one-shot mDNS implementation.
    pub fn build_query_message(&self) -> Option<DnsMessage> {
        if self.own_map_record.is_empty() {
            None
        } else {
            let mut questions = Vec::new();
            for (_netid, records) in self.own_map_record.iter() {
                for rec in records {
                    questions.push(rec.to_question_section());
                }
            }

            Some(DnsMessage {
                nb_questions: questions.len() as u16,
                questions,
                ..Default::default()
            })
        }
    }

    /// Sends unsolicited mDNS responses containing our node's resource records in the "Answer
    /// Section" of a DNS packet.
    fn announcing(&mut self) -> MulticastDnsResult<()> {
        let own_net_id_list = self.own_networkids();

        if let Some(dmesg) = self
            .own_map_record
            .to_dns_response_message(&own_net_id_list)
        {
            // Sends at least 2 time an unsolicited response, up to 8 times maximum (according to
            // the standard https://tools.ietf.org/html/rfc6762#section-8.3)
            self.broadcast_message(&dmesg)?;
            self.broadcast_message(&dmesg)?;
            self.broadcast_message(&dmesg)?;
        }
        Ok(())
    }
}

impl Discovery for MulticastDns {
    /// Make yourself known on the network.
    fn advertise(&mut self) -> DiscoveryResult<()> {
        self.query()?;
        self.announcing()?;
        Ok(())
    }

    /// Read the UDP stack and update our cache accordingly.
    fn discover(&mut self) -> DiscoveryResult<Vec<Lib3hUri>> {
        self.responder()?;

        // We should query (and announce in the same time because we will anwser to our query in the
        // next iteration) "every amount of time"
        if self.timestamp.elapsed().as_millis() > self.query_interval_ms {
            self.query()?;
            self.timestamp = Instant::now();
        }

        self.update_ttl();
        self.prune_cache();

        Ok(self.urls())
    }

    /// Release itself from the available participants in a network.
    fn release(&mut self) -> DiscoveryResult<()> {
        for (_netid, records) in self.own_map_record.iter_mut() {
            for rec in records.iter_mut() {
                // Since we want to leave the network space, we set our "time to live" to zero and let
                // other know about it
                rec.ttl = 0;
            }
        }

        let net_ids = self.own_networkids();
        if let Some(release_dmesg) = self.own_map_record.to_dns_response_message(&net_ids) {
            self.broadcast_message(&release_dmesg)?;
            self.broadcast_message(&release_dmesg)?;
            self.broadcast_message(&release_dmesg)?;
            self.broadcast_message(&release_dmesg)?;
        }

        Ok(())
    }

    /// Clear our cache from resource records.
    fn flush(&mut self) -> DiscoveryResult<()> {
        self.map_record.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_loop_question() {
        let mut mdns = MulticastDnsBuilder::new()
            .bind_address("0.0.0.0")
            .multicast_address("224.0.0.247")
            .bind_port(55247)
            .multicast_loop(true)
            .multicast_ttl(255)
            .build()
            .expect("build fail");

        let mut dmesg = DnsMessage::new();
        dmesg.nb_questions = 1;
        dmesg.questions = vec![QuerySection::new("lib3h.test.service")];

        // Let's empty the UDP socket stack from other test packets before sending our test packet
        let _ = mdns.recv().expect("Fail to receive from the UDP socket.");
        let _ = mdns.recv().expect("Fail to receive from the UDP socket.");

        mdns.broadcast_message(&dmesg)
            .expect("Fail to broadcast DNS Message.");

        // std::thread::sleep(std::time::Duration::from_millis(100));
        if let Some((resp, _addr)) = mdns.recv().expect("Fail to receive from the UDP socket.") {
            let dmesg_from_resp = DnsMessage::from_raw(&resp).unwrap();
            assert_eq!(
                &dmesg_from_resp.questions[0].domain_name,
                "lib3h.test.service"
            );
        }
    }

    #[test]
    fn it_should_loop_answer() {
        let mut mdns = MulticastDnsBuilder::new()
            .bind_address("0.0.0.0")
            .multicast_address("224.0.0.248")
            .bind_port(56248)
            .multicast_loop(true)
            .multicast_ttl(255)
            .build()
            .expect("build fail");

        let mut dmesg = DnsMessage::new();
        let answers = vec![
            AnswerSection::new("holonaute.local.", &Target::new("wss://192.168.0.88")),
            AnswerSection::new("mistral.local.", &Target::new("wss://192.168.0.77")),
        ];
        dmesg.nb_answers = answers.len() as u16;
        dmesg.answers = answers;

        // Let's empty the UDP socket stack from other test packets before sending our test packet
        let _ = mdns.recv().expect("Fail to receive from the UDP socket.");
        let _ = mdns.recv().expect("Fail to receive from the UDP socket.");
        let _ = mdns.recv().expect("Fail to receive from the UDP socket.");

        mdns.broadcast_message(&dmesg)
            .expect("Fail to broadcast DNS Message.");

        if let Some((resp, _addr)) = mdns.recv().expect("Fail to receive from the UDP socket.") {
            let dmesg_from_resp = DnsMessage::from_raw(&resp).unwrap();
            println!("dmesg = {:#?}", &dmesg);
            println!("dmesg_from_resp = {:#?}", &dmesg_from_resp);

            assert_eq!(dmesg, dmesg_from_resp);
        }
    }

    /// Tests if we can release ourself from the network.
    #[test]
    fn release_test() {
        // Let's share the same NetworkId, meaning we are on the same network.
        let networkid = "holonaute-release.holo.host";

        // This is the one from which we want to see another node disapearing from its cache
        let mut mdns = MulticastDnsBuilder::new()
            .own_record(networkid, &["wss://192.168.0.88:88088?a=to-keep"])
            .multicast_address("224.0.0.251")
            .bind_port(8251)
            .build()
            .expect("Fail to build mDNS.");

        let mut mdns_releaser = MulticastDnsBuilder::new()
            .own_record(networkid, &["wss://192.168.0.87:88088?a=to-release"])
            .multicast_address("224.0.0.251")
            .bind_port(8251)
            .build()
            .expect("Fail to build mDNS.");

        // Make itself known in the network
        mdns_releaser
            .advertise()
            .expect("Fail to advertise my existence during release test.");
        ::std::thread::sleep(::std::time::Duration::from_millis(100));

        // Discovering the soon-to-be-leaving participant
        mdns.discover().expect("Fail to discover.");
        ::std::thread::sleep(::std::time::Duration::from_millis(100));

        println!("mdns = {:#?}", &mdns.map_record);
        // Let's check that we discovered the soon-to-be-released record
        {
            let records = mdns
                .map_record
                .get(networkid)
                .expect("Fail to get records from the networkid after 'Advertising'.");
            assert_eq!(records.len(), 2);
        }

        // Leaving the party
        mdns_releaser
            .release()
            .expect("Fail to release myself from the participants on the network.");
        ::std::thread::sleep(::std::time::Duration::from_millis(100));

        // Updating the cache
        mdns.discover().expect("Fail to discover.");
        ::std::thread::sleep(::std::time::Duration::from_millis(100));

        println!("mdns = {:#?}", &mdns.map_record);
        {
            let records = mdns
                .map_record
                .get(networkid)
                .expect("Fail to get records from the networkid after 'Releasing'.");
            assert_eq!(records.len(), 1);
        }
    }

    /// Tests if we are able to query info to other peer on the network for our NetworkId.
    #[test]
    fn query_test() -> MulticastDnsResult<()> {
        // Let's share the same NetworkId, meaning we are on the same network.
        let networkid = "holonaute-query.holo.host";

        let mut mdns_actor1 = MulticastDnsBuilder::new()
            .own_record(networkid, &["wss://192.168.0.88:88088?a=hc-actor1"])
            .multicast_address("224.0.0.223")
            .bind_port(8223)
            .query_interval_ms(1)
            .build()
            .expect("Fail to build mDNS.");

        let mut mdns_actor2 = MulticastDnsBuilder::new()
            .own_record(networkid, &["wss://192.168.0.87:88088?a=hc-actor2"])
            .multicast_address("224.0.0.223")
            .bind_port(8223)
            .query_interval_ms(1)
            .build()
            .expect("Fail to build mDNS.");

        // We want to make sure that that client1 can discover himself and client2

        // We should not need to advertise, query should be enough
        mdns_actor1.query()?;
        ::std::thread::sleep(::std::time::Duration::from_millis(10));
        mdns_actor1.query()?;
        ::std::thread::sleep(::std::time::Duration::from_millis(10));
        mdns_actor1.discover()?;
        ::std::thread::sleep(::std::time::Duration::from_millis(10));
        mdns_actor1.discover()?;
        // At this point mdns_actor1 should know about himself
        let records = mdns_actor1
            .map_record
            .get(networkid)
            .expect("Fail to get records from the networkid during Query test on mdns_actor1")
            .to_vec();
        assert_eq!(records.len(), 1);
        eprintln!("mdns_actor1 = {:#?}", &mdns_actor1.map_record);

        // Let's do the same for the second actor
        mdns_actor2.query()?;
        ::std::thread::sleep(::std::time::Duration::from_millis(10));
        mdns_actor2.discover()?;
        // At this point mdns_actor2 should know about himself
        let mut records = mdns_actor2
            .map_record
            .get(networkid)
            .expect("Fail to get records from the networkid during Query test on mdns_actor2")
            .to_vec();
        eprintln!("mdns_actor2 = {:#?}", &mdns_actor2.map_record);

        // Make the order deterministic
        records.sort_by(|a, b| a.url.cmp(&b.url));

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].url, "wss://192.168.0.87:88088?a=hc-actor2");
        assert_eq!(records[1].url, "wss://192.168.0.88:88088?a=hc-actor1");

        Ok(())
    }

    /// Tests if we are able to make ourselves known on the network.
    #[test]
    fn advertise_test() {
        // Let's share the same NetworkId, meaning we are on the same network.
        let networkid = "holonaute-advertise.holo.host";

        // This is the one from which we want to see another node disapearing from its cache
        let mut mdns_actor1 = MulticastDnsBuilder::new()
            .own_record(networkid, &["wss://192.168.0.88:88088?a=hc-actor1"])
            .multicast_address("224.0.0.252")
            .bind_port(8252)
            .build()
            .expect("Fail to build mDNS.");

        eprintln!("bind addr = {}", mdns_actor1.multicast_address());

        let mut mdns_actor2 = MulticastDnsBuilder::new()
            .own_record(networkid, &["wss://192.168.0.88:88088?a=hc-actor2"])
            .multicast_address("224.0.0.252")
            .bind_port(8252)
            .build()
            .expect("Fail to build mDNS.");

        // Make itself known on the network
        mdns_actor2
            .advertise()
            .expect("Fail to advertise mdns_actor1 existence during release test.");
        ::std::thread::sleep(::std::time::Duration::from_millis(10));

        mdns_actor1
            .advertise()
            .expect("Fail to advertise mdns_actor2 existence during release test.");
        ::std::thread::sleep(::std::time::Duration::from_millis(10));

        // Discovering the soon leaving participant
        mdns_actor1.discover().expect("Fail to discover.");
        eprintln!("mdns = {:#?}", &mdns_actor1.map_record);

        // Let's check that we discovered the soon to release record
        let records = mdns_actor1
            .map_record
            .get(networkid)
            .expect("Fail to get records from the networkid");
        assert_eq!(records.len(), 2);
    }
}

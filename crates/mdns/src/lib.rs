//! lib3h mDNS LAN discovery module
//!
//! Our simple use case is the following:
//! ```rust
//! use lib3h_mdns as mdns;
//! use lib3h_discovery::Discovery;
//!
//! let mut mdns = mdns::MulticastDnsBuilder::new()
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
//!     mdns::sleep_ms(100);
//! }
//! ```

#![feature(try_trait)]
#![feature(never_type)]

use log::{debug, error, trace, warn};
use rand::Rng;
use regex;
use std::net;

#[cfg(not(target_os = "windows"))]
use net2::unix::UnixUdpBuilderExt;

use std::{
    net::{SocketAddr, ToSocketAddrs},
    thread,
    time::{Duration, Instant},
};

use lib3h_discovery::{error::DiscoveryResult, Discovery};

pub mod error;
pub use error::{MulticastDnsError, MulticastDnsResult};

pub mod dns;
pub use dns::*;

pub mod record;
use record::{HashMapRecord, MapRecord, Record};

// 20 byte IP header would mean 65_507... but funky configs can increase that
// const READ_BUF_SIZE: usize = 60_000;
// however... we don't want to accept any packets that big...
// let's stick with one common block size
const READ_BUF_SIZE: usize = 4_096;

/// Delay between probe query, 250ms by default.
const PROBE_QUERY_DELAY_MS: u64 = 250;

/// Listening port of this mDNS service.
const SERVICE_LISTENER_PORT: u16 = 8585;

/// mDNS multicast IPv4 address.
const MDNS_MULCAST_IPV4_ADRESS: &str = "224.0.0.251";

/// Default bind adress.
const DEFAULT_BIND_ADRESS: &str = "0.0.0.0";

/// mdns builder
pub struct MulticastDnsBuilder {
    pub(crate) bind_address: String,
    pub(crate) bind_port: u16,
    pub(crate) multicast_loop: bool,
    pub(crate) multicast_ttl: u32,
    pub(crate) multicast_address: String,
    pub(crate) own_record: Record,
}

impl MulticastDnsBuilder {
    /// create a new mdns builder
    pub fn new() -> Self {
        MulticastDnsBuilder::default()
    }

    /// specify the network interface to bind to
    pub fn bind_address(&mut self, addr: &str) -> &mut Self {
        self.bind_address = addr.to_owned();
        self
    }

    /// specify the udp port to listen on
    pub fn bind_port(&mut self, port: u16) -> &mut Self {
        self.bind_port = port;
        self
    }

    /// should we loop broadcasts back to self?
    pub fn multicast_loop(&mut self, should_loop: bool) -> &mut Self {
        self.multicast_loop = should_loop;
        self
    }

    /// set the multicast ttl
    pub fn multicast_ttl(&mut self, ttl: u32) -> &mut Self {
        self.multicast_ttl = ttl;
        self
    }

    /// set the multicast address
    pub fn multicast_address(&mut self, addr: &str) -> &mut Self {
        self.multicast_address = addr.to_string();
        self
    }

    /// Set the host's record.
    pub fn own_record(&mut self, hostname: &str, addrs: &[&str]) -> &mut Self {
        let addrs: Vec<String> = addrs.iter().map(|a| a.to_string()).collect();
        let hostname = hostname.split_terminator(".local.").collect::<Vec<&str>>()[0];
        self.own_record = Record::new(hostname, &addrs, 255);
        self
    }

    /// construct the actual mdns struct
    pub fn build(&mut self) -> Result<MulticastDns, MulticastDnsError> {
        let recv_socket = create_socket(&self.bind_address, self.bind_port)?;
        recv_socket.set_nonblocking(true)?;
        recv_socket.set_multicast_loop_v4(self.multicast_loop)?;
        recv_socket.set_multicast_ttl_v4(self.multicast_ttl)?;
        recv_socket.join_multicast_v4(
            &self.multicast_address.parse()?,
            &self.bind_address.parse()?,
        )?;

        let send_socket = create_socket(
            DEFAULT_BIND_ADRESS,
            get_available_port(DEFAULT_BIND_ADRESS)?,
        )?;
        send_socket.set_nonblocking(true)?;

        Ok(MulticastDns {
            bind_address: self.bind_address.to_owned(),
            bind_port: self.bind_port,
            multicast_loop: self.multicast_loop,
            multicast_ttl: self.multicast_ttl,
            multicast_address: self.multicast_address.to_owned(),
            send_socket,
            recv_socket,
            buffer: [0; READ_BUF_SIZE],
            own_record: self.own_record.clone(),
            map_record: MapRecord {
                value: HashMapRecord::with_capacity(32),
            },
        })
    }
}

use std::default::Default;
impl Default for MulticastDnsBuilder {
    fn default() -> Self {
        MulticastDnsBuilder {
            bind_address: String::from(DEFAULT_BIND_ADRESS),
            bind_port: SERVICE_LISTENER_PORT,
            multicast_loop: true,
            multicast_ttl: 255,
            multicast_address: String::from(MDNS_MULCAST_IPV4_ADRESS),
            own_record: Record::new_own(),
        }
    }
}

fn get_available_port(addr: &str) -> MulticastDnsResult<u16> {
    for port in SERVICE_LISTENER_PORT + 1..65535 {
        if net::UdpSocket::bind((addr, port)).is_ok() {
            return Ok(port);
        }
    }
    Err(MulticastDnsError::new(
        crate::error::ErrorKind::NoAvailablePort,
    ))
}

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
    /// The socket used by the mDNS service protocol to send packets
    pub(crate) send_socket: net::UdpSocket,
    /// The socket used to receive mDNS packets
    pub(crate) recv_socket: net::UdpSocket,
    /// The buffer used to store the packet to send/receive messages
    buffer: [u8; READ_BUF_SIZE],
    /// Reference the host's record
    pub(crate) own_record: Record,
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

    /// Insert a new record to our cache.
    pub fn push_record(&mut self, hostname: &str, record: &Record) {
        self.map_record.insert(hostname.to_string(), record.clone());
    }

    /// Update our cache of resource records.
    pub fn update_cache(&mut self, other_map_record: &MapRecord) {
        for (other_name, other_record) in other_map_record.iter() {
            if self.own_record.hostname != *other_name {
                self.map_record
                    .insert(other_name.clone(), other_record.clone());
            }
        }
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
        self.flush_buffer();

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

    /// Try to receive a DNS packet and set timeout and wait time doing so.
    pub fn recv_probe_response(
        &mut self,
        timeout: u64,
        every: u64,
    ) -> Result<Option<(AnswerSection, SocketAddr)>, MulticastDnsError> {
        self.flush_buffer();
        let start = Instant::now();

        while start.elapsed().as_millis() < u128::from(timeout) {
            match self.recv_socket.recv_from(&mut self.buffer) {
                Ok((num_bytes, sender_socket_addr)) => {
                    let packet = self.buffer[..num_bytes].to_vec();
                    trace!("I received '{}' bytes: {:?}", num_bytes, &packet);

                    if let Ok(dmesg) = DnsMessage::from_raw(&packet) {
                        // We discard non authoritative answers
                        if dmesg.nb_authority < 1 {
                            continue;
                        } else {
                            for answer in dmesg.answers.iter() {
                                if self.own_record.hostname() == answer.domain_name {
                                    return Ok(Some((answer.clone(), sender_socket_addr)));
                                }
                            }
                        }
                    } else {
                        warn!("Fail to cast DnsMessage from : {:?}", &packet);
                    }
                }
                Err(err) => {
                    if err.kind() != std::io::ErrorKind::WouldBlock {
                        error!("Something went wrong: {}", err);
                        return Err(err.into());
                    } else {
                    }
                }
            }
            sleep_ms(every);
        }
        Ok(None)
    }

    /// Clean our buffer.
    pub fn flush_buffer(&mut self) {
        for i in 0..self.buffer.len() {
            self.buffer[i] = 0;
        }
    }

    /// Clean our cache by removing the out of live records.
    pub fn clean_cache(&mut self) {
        // Get the entry of the dead records to remove them safely afterward
        let mut dead_entry_list: Vec<String> = Vec::with_capacity(self.map_record.len());
        for (k, v) in self.map_record.iter() {
            if v.ttl == 0 {
                dead_entry_list.push(k.clone());
            }
        }

        // Safely clean our cache from dead entry
        for dead_entry in dead_entry_list {
            self.map_record.remove(&dead_entry);
        }
    }

    /// Run the mDNS service.
    pub fn run(&mut self) -> MulticastDnsResult<!> {
        println!("Startuping.");
        self.advertise()?;

        // Run this async
        println!("Responder service started.");
        self.responder()?;

        // Ok(())
    }

    /// mDNS Querier
    /// One-Shot Multicast DNS Queries
    /// Not used in our actual mDNS implementation use case.
    pub fn query(&mut self) -> MulticastDnsResult<()> {
        // if let Some(query_message) = self.build_query_message() {
        //     self.broadcast(&query_message)?;
        // }

        Ok(())
    }

    /// A mDNS Responder that listen to the network in order to defend its name and respond to
    /// queries
    /// Not used in our actual mDNS implementation use case.
    fn responder(&mut self) -> MulticastDnsResult<!> {
        let defend_message = self.build_defensive_message();

        loop {
            match self.recv() {
                Ok(Some((packet, sender_addr))) => {
                    let dmesg = DnsMessage::from_raw(&packet)?;
                    if dmesg.nb_answers > 0 {
                        // Skipping for now...
                    }
                    // We send response only for record we have authority on.
                    // We send the response directly to the sender instead of broadcasting it to
                    // avoid any unnecessary burden on the network.
                    if dmesg.nb_questions > 0 {
                        for question in dmesg.questions.iter() {
                            if question.domain_name == self.own_record.hostname {
                                // Apparently this fail on a local network, so we broadcast it
                                // anyway
                                self.send_socket
                                    .send_to(&defend_message.to_raw()?, sender_addr)?;
                                self.broadcast_message(&defend_message)?;
                            }
                        }
                    }
                }
                Ok(None) => {
                    debug!(">> Nothing on the UDP stack");
                    // break;
                }
                Err(e) => {
                    error!(
                        "Something went wrong while processing the UDP stack during update: '{}'",
                        e
                    );
                    // break;
                    return Err(e);
                }
            }

            thread::sleep(Duration::from_millis(1000));
        }
        // Ok(())
    }

    /// Builds mDNS probe packet with the proper bit set up in order to check if a host record is
    /// available.
    fn build_probe_packet(&self) -> DnsMessage {
        let questions = vec![QuerySection::new(&self.own_record.hostname)];
        DnsMessage {
            nb_questions: 1,
            questions,
            ..Default::default()
        }
    }

    /// Builds an mDNS reponse packet containing all the registered ressource records of the host
    /// in the "Answer Section".
    pub fn build_response_packet(&self, record: &Record) -> DnsMessage {
        let resp_record = self
            .map_record
            .get(record.hostname())
            .expect("Missing record.");
        let mr = MapRecord::new(&resp_record.hostname, &resp_record);
        mr.to_dns_reponse_message()
    }

    /// Build a packet to defend our name.
    pub fn build_defensive_message(&self) -> DnsMessage {
        let mut def_mesg =
            MapRecord::new(&self.own_record.hostname, &self.own_record).to_dns_reponse_message();
        def_mesg.nb_authority = 1;
        def_mesg
    }

    pub fn build_release_message(&self) -> DnsMessage {
        // let mut release_record = self.own_record.clone();
        // release_record.ttl = 0;
        // MapRecord::new(&self.own_record.hostname, &release_record).to_dns_reponse_message()
        MapRecord::new(&self.own_record.hostname, &self.own_record).to_dns_reponse_message()
    }

    /// Builds an announcing packet corresponding to an unsolicited mDNS response containing all of
    /// the node's cache.
    fn build_announcing_message(&self) -> DnsMessage {
        self.map_record.to_dns_reponse_message()
    }

    /// Builds a query packet to be used by one-shot mDNS implementation.
    pub fn build_query_message(&self, hostname: &str) -> Option<DnsMessage> {
        if let Some(record) = self.map_record.get(hostname) {
            let questions = vec![QuerySection::new(&record.hostname)];
            Some(DnsMessage {
                nb_questions: 1,
                questions,
                ..Default::default()
            })
        } else {
            None
        }
    }

    /// When sending probe queries, a host MUST NOT consult its cache for
    /// potential answers.  Only conflicting Multicast DNS responses received
    /// "live" from the network are considered valid for the purposes of
    /// determining whether probing has succeeded or failed.
    fn probe(&mut self) -> MulticastDnsResult<()> {
        // Making sure our cache is empty before probing
        self.map_record.clear();

        // Let's wait a moment to give time to other nodes to initialize their network
        let delay_probe_by: u64 = rand_delay(0, PROBE_QUERY_DELAY_MS);
        sleep_ms(delay_probe_by);

        // Abitrary value to prevent any infinite loop if there is some mischief happening on the
        // network
        let mut fail_safe = 0;
        let mut retry = 0;
        while retry < 3 && fail_safe < 1000 {
            let probe_packet = self.build_probe_packet();
            fail_safe += 1;
            retry += 1;
            self.broadcast_message(&probe_packet)?;

            // If this function returns, it means that a conflict has been detected
            if let Some((_answer, _sender_socket_addr)) = self.recv_probe_response(3_000, 10)? {
                self.resolve_conflict()?;
                retry = 0;
            }
        }

        if retry == 3 && self.map_record.is_empty() {
            // Nobody complained on the network about our hostname,
            // so let's take authority on our hostname and update our cache accordingly
            self.map_record
                .insert(self.own_record.hostname.clone(), self.own_record.clone());
        } else if fail_safe == 1000 {
            panic!("Fail safe triggered during probe step. Something is wrong in the network Sir.");
        }

        // Set back the read time of the receiving socket
        self.recv_socket
            .set_read_timeout(Some(Duration::from_millis(5_000)))?;
        self.recv_socket.set_nonblocking(true)?;
        Ok(())
    }

    /// Sends unsolicited mDNS responses containing our node's resource records in the "Answer
    /// Section" of a DNS packet.
    fn announcing(&mut self) -> MulticastDnsResult<()> {
        let dmesg = self.build_announcing_message();

        // Sends at least 2 time an unsolicited response, up to 8 times maximum
        for _ in 0..rand_delay(2_usize, 9_usize) {
            // Sends mDNS responses containing all of its resource records in the "Answer Section"
            self.broadcast_message(&dmesg)?;
            sleep_ms(1_000);
        }
        Ok(())
    }

    /// Detects confict: check if a record already exist in the cache.
    pub fn detect_conflict_during_probe(&self, other_record: &Record) -> bool {
        if self.map_record.is_empty() {
            false
        } else {
            self.map_record.get(&other_record.hostname).is_some()
        }
    }

    /// Resolve a conflict by renaming our own record in the case of conflict.
    pub fn resolve_conflict(&mut self) -> MulticastDnsResult<String> {
        let hostname = self.own_record.hostname();
        // Take only what we want
        let base_hostname = hostname.split_terminator(".local.").collect::<Vec<&str>>()[0];

        let re = regex::Regex::new(r"(\.)(\d$)")?;
        let confict_free_hostname = match re.captures(&base_hostname) {
            Some(cap) => {
                let nb = cap.get(2)?.as_str().parse::<i32>().unwrap_or(0) + 1;
                let suffix = cap.get(0)?.as_str();

                let base_hostname = base_hostname
                    .split_terminator(suffix)
                    .collect::<Vec<&str>>()[0];
                format!("{}.{}.local.", &base_hostname, nb)
            }
            None => format!("{}.{}.local.", &base_hostname, 1),
        };

        self.own_record.hostname = confict_free_hostname.clone();
        Ok(confict_free_hostname)
    }
}

impl Discovery for MulticastDns {
    /// Make yourself known on the network.
    fn advertise(&mut self) -> DiscoveryResult<()> {
        for _retry in 0..15 {
            match self.probe() {
                Ok(_) => break,
                Err(ref err) => match err.kind() {
                    error::ErrorKind::ProbeError => {
                        sleep_ms(1_000);
                    }
                    _ => panic!("Unrecoverable error encountered during probe step."),
                },
            }
        }

        self.announcing().expect("Fail to announce during startup.");
        Ok(())
    }

    /// Read the UDP stack and update our cache accordingly.
    fn discover(&mut self) -> DiscoveryResult<()> {
        // Process all element of the UDP socket stack
        loop {
            match self.recv() {
                Ok(Some((packet, sender_addr))) => {
                    let dmesg = DnsMessage::from_raw(&packet)?;
                    if dmesg.nb_answers > 0 {
                        if let Some(new_map_record) = MapRecord::from_dns_message(&dmesg) {
                            self.update_cache(&new_map_record);
                        }
                    }
                    // We send response only for record we have authority on.
                    // We send the response directly to the sender instead of broadcasting it to
                    // avoid any unnecessary burden on the network.
                    if dmesg.nb_questions > 0 {
                        for question in dmesg.questions.iter() {
                            if question.domain_name == self.own_record.hostname {
                                let response = self.build_defensive_message();
                                self.send_socket.send_to(&response.to_raw()?, sender_addr)?;
                            }
                        }
                    }
                }
                Ok(None) => {
                    debug!(">> Nothing on the UDP stack");
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

        self.announcing()?;

        for (name, record) in self.map_record.iter_mut() {
            if record.ttl > 1 && name != self.own_record.hostname() {
                record.ttl -= 1;
            }
        }

        self.clean_cache();

        Ok(())
    }

    /// Release itself from the available participants in a network.
    fn release(&mut self) -> DiscoveryResult<()> {
        self.own_record.ttl = 0;
        self.map_record
            .insert(self.own_record.hostname.clone(), self.own_record.clone());
        self.broadcast_message(&self.build_release_message())
            .expect("Fail to broadcast release message.");

        Ok(())
    }

    /// Clear our cache from resource records.
    fn flush(&mut self) -> DiscoveryResult<()> {
        self.map_record.clear();
        Ok(())
    }
}

/// non-windows udp socket bind.
#[cfg(not(target_os = "windows"))]
fn create_socket(addr: &str, port: u16) -> Result<std::net::UdpSocket, MulticastDnsError> {
    Ok(net2::UdpBuilder::new_v4()?
        .reuse_address(true)?
        .reuse_port(true)?
        .bind((addr, port))?)
}

/// windows udp socket bind.
#[cfg(target_os = "windows")]
fn create_socket(addr: &str, port: u16) -> Result<std::net::UdpSocket, MulticastDnsError> {
    Ok(net2::UdpBuilder::new_v4()?
        .reuse_address(true)?
        .bind((addr, port))?)
}

pub fn sleep_ms(duration: u64) {
    thread::sleep(Duration::from_millis(duration));
}

/// Wrapper around randomized range generation.
fn rand_delay<T>(low: T, high: T) -> T
where
    T: rand::distributions::uniform::SampleUniform,
{
    rand::thread_rng().gen_range(low, high)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_loop_question() {
        let mut mdns = MulticastDnsBuilder::new()
            .bind_address("0.0.0.0")
            .bind_port(55055)
            .multicast_loop(true)
            .multicast_ttl(255)
            .multicast_address("224.0.0.247")
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
            .bind_port(55055)
            .multicast_loop(true)
            .multicast_ttl(255)
            .multicast_address("224.0.0.248")
            .build()
            .expect("build fail");

        let mut dmesg = DnsMessage::new();
        let answers = vec![
            AnswerSection::new("holonaute.local.", &[Target::new("wss://192.168.0.88")]),
            AnswerSection::new("mistral.local.", &[Target::new("wss://192.168.0.77")]),
        ];
        dmesg.nb_answers = answers.len() as u16;
        dmesg.answers = answers;

        // Let's empty the UDP socket stack from other test packets before sending our test packet
        let _ = mdns.recv().expect("Fail to receive from the UDP socket.");
        let _ = mdns.recv().expect("Fail to receive from the UDP socket.");
        let _ = mdns.recv().expect("Fail to receive from the UDP socket.");

        mdns.broadcast_message(&dmesg)
            .expect("Fail to broadcast DNS Message.");

        // std::thread::sleep(std::time::Duration::from_millis(100));
        if let Some((resp, _addr)) = mdns.recv().expect("Fail to receive from the UDP socket.") {
            let dmesg_from_resp = DnsMessage::from_raw(&resp).unwrap();
            println!("dmesg = {:#?}", &dmesg);
            println!("dmesg_from_resp = {:#?}", &dmesg_from_resp);

            assert_eq!(dmesg, dmesg_from_resp);
        }
    }

    #[test]
    fn probe_message_test() {
        let mdns = MulticastDnsBuilder::new()
            .bind_address("0.0.0.0")
            .bind_port(56056)
            .multicast_loop(true)
            .multicast_ttl(255)
            .multicast_address("224.0.0.246")
            .build()
            .expect("build fail");

        let probe_message = mdns.build_probe_packet();

        let probe_message_from_raw = DnsMessage::from_raw(
            &probe_message
                .to_raw()
                .expect("Fail to convert probe message to bytes."),
        )
        .expect("Fail to convert back probe message from bytes.");

        assert_eq!(probe_message, probe_message_from_raw);
        assert_ne!(probe_message.nb_questions, 0);
    }

    #[test]
    fn resolve_conflict_test() {
        let resolve_conflict_name = {
            let mut mdns = MulticastDnsBuilder::new()
                .multicast_address("224.0.0.249")
                .build()
                .expect("Fail to build mDNS.");

            let own_record = Record::new_own();
            let resolve_conflict_name = mdns.resolve_conflict().expect("Fail to resolve confict.");
            assert_ne!(own_record.hostname(), resolve_conflict_name);
            resolve_conflict_name
        };

        let mut mdns = MulticastDnsBuilder::new()
            .own_record(&resolve_conflict_name, &["0.0.0.0"])
            .multicast_address("224.0.0.249")
            .build()
            .expect("Fail to build mDNS.");

        let own_record = Record::new_own();
        let resolve_confict_name = mdns.resolve_conflict().expect("Fail to resolve confict.");
        assert_ne!(own_record.hostname(), resolve_confict_name);

        let mut mdns = MulticastDnsBuilder::new()
            .own_record("asgard.1.2", &["0.0.0.0"])
            .multicast_address("224.0.0.249")
            .build()
            .expect("Fail to build mDNS.");

        let own_record = Record::new_own();
        let resolve_confict_name = mdns.resolve_conflict().expect("Fail to resolve confict.");
        assert_ne!(own_record.hostname(), resolve_confict_name);
    }

    #[test]
    fn responder_and_conflict_resolver_test() {
        use std::sync::mpsc::channel;

        let (sx, rx) = channel();

        let _handle = thread::Builder::new()
            .name(String::from("mDNS Responder"))
            .spawn(move || {
                let mut mdns = MulticastDnsBuilder::new()
                    .own_record("holonaute", &["0.0.0.0"])
                    .bind_port(8596)
                    .multicast_address("224.0.0.252")
                    .build()
                    .expect("Fail to build mDNS.");

                // Skip the advertising step
                let own_record = Record::new("holonaute.local.", &["0.0.0.0".to_string()], 255);
                mdns.map_record
                    .insert("holonaute.local.".to_string(), own_record);

                sx.send(mdns.own_record.clone())
                    .expect("Fail to send mDNS service through channel.");

                // Listen to the network for a few moment, just the time to defend our name
                mdns.responder()
                    .expect("Fail to fire up the mDNS responder service.");

                // eprintln!("Exit defending thread.");
            });

        let mdns_own_record = rx
            .recv()
            .expect("Fail to receive mDNS server through channel.");

        drop(_handle);
        drop(rx);

        let mut mdns_with_resolved_conflict = MulticastDnsBuilder::new()
            .own_record("holonaute", &["0.0.0.0"])
            .bind_port(8596)
            .multicast_address("224.0.0.252")
            .build()
            .expect("Fail to build mDNS.");

        mdns_with_resolved_conflict
            .advertise()
            .expect("Fail to advertise the conflictual mDNS server.");

        assert_ne!(mdns_own_record, mdns_with_resolved_conflict.own_record);
    }

    #[test]
    fn release_test() {
        let own_record = Record::new("holonaute.local.", &["0.0.0.0".to_string()], 255);
        // This is the one from which we want to see another node disapearing from its cache
        let mut mdns = MulticastDnsBuilder::new()
            .multicast_address("224.0.0.251")
            .own_record("holonaute", &["0.0.0.0"])
            .build()
            .expect("Fail to build mDNS.");

        mdns.map_record
            .insert("holonaute.local.".to_string(), own_record);

        let mut mdns_releaser = MulticastDnsBuilder::new()
            .own_record("holonaute-to-release", &["0.0.0.0"])
            .multicast_address("224.0.0.251")
            .build()
            .expect("Fail to build mDNS.");

        // Make itself known ion the network
        mdns_releaser
            .advertise()
            .expect("Fail to advertise my existence during release test.");

        // Discovering the soon leaving participant
        mdns.discover().expect("Fail to discover.");

        // Leaving the party
        mdns_releaser
            .release()
            .expect("Fail to release myself from the participants on the network.");

        // Updating the cache
        mdns.discover().expect("Fail to discover.");

        assert_eq!(mdns.map_record.get("holonaute-to-release.local."), None);
    }
}

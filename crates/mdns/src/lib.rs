//! lib3h mDNS LAN discovery module

#![feature(try_trait)]
#![feature(never_type)]

use rand::Rng;
// use byteorder;
// use hostname;
// use net2::UdpSocketExt;
use regex;
use std::net;

#[cfg(not(target_os = "windows"))]
use net2::unix::UnixUdpBuilderExt;

use std::{
    collections::HashMap,
    net::{SocketAddr, ToSocketAddrs},
    thread,
    time::{Duration, Instant},
};

pub mod error;
pub use error::{MulticastDnsError, MulticastDnsResult};

pub mod dns;
pub use dns::{Answer, Packet, Question, SrvDataA, SrvDataQ};

pub mod protocol;
use protocol::Discovery;

pub mod record;
use record::{MapRecord, Record};

// 20 byte IP header would mean 65_507... but funky configs can increase that
// const READ_BUF_SIZE: usize = 60_000;
// however... we don't want to accept any packets that big...
// let's stick with one common block size
const READ_BUF_SIZE: usize = 4_096;

/// Delay between probe query, 250ms by default.
const PROBE_QUERY_DELAY_MS: u64 = 250;

/// Listening port of this mDNS service.
const SERVICE_LISTENER_PORT: u16 = 8585;

// /// Type helper corresponding to the resource record of a host.
// type Records = HashMap<String, Record>;

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
    pub fn own_record(&mut self, hostname: &str, addr: &[&str]) -> &mut Self {
        let addr: Vec<net::Ipv4Addr> = addr
            .iter()
            .map(|ip| ip.parse().expect("Fail to parse IPv4 String address."))
            .collect();
        let hostname = hostname.split_terminator(".local.").collect::<Vec<&str>>()[0];
        self.own_record = Record::new(hostname, &addr);
        self
    }

    /// construct the actual mdns struct
    pub fn build(&mut self) -> Result<MulticastDns, MulticastDnsError> {
        let recv_socket = create_socket(&self.bind_address, self.bind_port)?;
        // recv_socket.set_nonblocking(false)?;
        recv_socket.set_nonblocking(true)?;
        recv_socket.set_multicast_loop_v4(self.multicast_loop)?;
        recv_socket.set_multicast_ttl_v4(self.multicast_ttl)?;
        recv_socket.join_multicast_v4(
            &self.multicast_address.parse()?,
            &self.bind_address.parse()?,
        )?;

        let send_socket = create_socket("0.0.0.0", get_available_port("0.0.0.0")?)?;
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
            map_record: HashMap::with_capacity(32),
        })
    }
}

use std::default::Default;
impl Default for MulticastDnsBuilder {
    fn default() -> Self {
        MulticastDnsBuilder {
            bind_address: String::from("0.0.0.0"),
            bind_port: SERVICE_LISTENER_PORT,
            multicast_loop: true,
            multicast_ttl: 255,
            multicast_address: String::from("224.0.0.251"),
            own_record: Record::new_own(),
        }
    }
}

fn get_available_port(addr: &str) -> MulticastDnsResult<u16> {
    for port in SERVICE_LISTENER_PORT+1..65535 {
        match net::UdpSocket::bind((addr, port)) {
            Ok(_) => return Ok(port),
            _ => (),
        }
    }
    Err(MulticastDnsError::new_other(
        "Fail to get an ephemeral mDNS sending port.",
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

    /// Returns the lookup table of records as a [HashMap]
    pub fn records(&self) -> &MapRecord {
        &self.map_record
    }

    /// Insert a new record to our cache.
    pub fn push_record(&mut self, hostname: &str, record: &Record) {
        self.map_record.insert(hostname.to_string(), record.clone());
    }

    /// Update our cache of resource records.
    pub fn update_cache(&mut self, records: &MapRecord) {
        dbg!(&records);
        for (_name, new_record) in records.iter() {
            if let Some(rec) = self.map_record.get(&new_record.hostname) {
                let new_addr = new_record.addrs.first().expect("Empty list of address.");
                let mut record_to_update = rec.clone();

                if !record_to_update.addrs.contains(new_addr) {
                    record_to_update.addrs.push(*new_addr);
                }
                self.map_record
                    .insert(new_record.hostname.clone(), record_to_update.clone());
            } else {
                self.map_record
                    .insert(new_record.hostname.clone(), new_record.clone());
            }
        }
    }

    /// broadcasts a dns packet.
    pub fn broadcast(&mut self, packet: &Packet) -> Result<usize, MulticastDnsError> {
        let addr = (self.multicast_address.as_ref(), self.bind_port)
            .to_socket_addrs()?
            .next()?;
        let data = packet.to_raw()?;

        Ok(self.send_socket.send_to(&data, &addr)?)
    }

    /// try to receive a dns packet.
    /// will return None rather than blocking if none are queued
    pub fn recv(&mut self) -> Result<Option<Packet>, MulticastDnsError> {
        self.clean_buffer();
        let (read, _) = match self.recv_socket.recv_from(&mut self.buffer) {
            Ok(r) => r,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    return Ok(None);
                }
                return Err(e.into());
            }
        };

        if read > 0 {
            let packet = Packet::from_raw(&self.buffer[0..read])?;
            Ok(Some(packet))
        } else {
            Ok(None)
        }
    }

    /// Try to receive a DNS packet and set timeout and wait time doing so.
    pub fn recv_timely(
        &mut self,
        timeout: u64,
        every: u64,
        for_duration: u64,
    ) -> Result<Option<(Packet, SocketAddr)>, MulticastDnsError> {
        self.clean_buffer();
        // Sets a timeout for other host to respond
        self.recv_socket
            .set_read_timeout(Some(Duration::from_millis(timeout)))?;

        let start = Instant::now();

        while start.elapsed().as_millis() < for_duration as u128 {
            match self.recv_socket.recv_from(&mut self.buffer) {
                Ok((num_bytes, sender_socket_addr)) => {
                    eprintln!(
                        "I received '{}' bytes: {:?}",
                        num_bytes,
                        &self.buffer.to_vec()[..num_bytes]
                    );
                    // TODO check if sender_addr match local link
                    let packet = Packet::from_raw(&self.buffer[..num_bytes])?;
                    return Ok(Some((packet, sender_socket_addr)));
                }
                Err(err) => {
                    if err.kind() != std::io::ErrorKind::WouldBlock {
                        eprintln!("Something went wrong: {}", err);
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
    pub fn clean_buffer(&mut self) {
        for i in 0..self.buffer.len() {
            self.buffer[i] = 0;
        }
    }

    // /// Startup phase corresponding to the
    // /// [probing](https://tools.ietf.org/html/rfc6762#section-8.1) and
    // /// [annoncing](https://tools.ietf.org/html/rfc6762#section-8.3) phases.
    // pub fn init(&mut self) -> MulticastDnsResult<()> {
    //     // Fires up the service listener
    //     ..
    //
    //     // Run the mDNS startup phase
    //     self.probe()?;
    //     self.announcing()?;
    //
    //     Ok(())
    // }

    /// Run the mDNS service.
    pub fn run(&mut self) -> MulticastDnsResult<()> {
        println!("Startuping.");
        self.startup();

        // Run this async
        println!("Responder service started.");
        self.responder()?;

        Ok(())
    }

    /// mDNS Querier
    /// One-Shot Multicast DNS Queries
    pub fn query(&mut self) -> MulticastDnsResult<()> {
        let query_packet = self.build_query_packet();
        // let query_packet = self.build_probe_packet();
        self.broadcast(&query_packet)?;

        for (name, record) in self.map_record.iter_mut() {
            if record.ttl > 1 && name != self.own_record.hostname() {
                record.ttl -= 1;
            }
        }

        // Set receive timeout to 2sec. And poll connection every 1sec for 3sec
        // let _response = self.recv_timely(2_000, 1_000, 3_000);

        // TODO handle response:
        // - detect confict
        // - if no conflict, update cache ?

        Ok(())
    }

    /// A mDNS Responder that listen to the network in order to defend its name and respond to
    /// queries
    fn responder(&mut self) -> MulticastDnsResult<!> {
        let resp_packet = self.build_defensive_packet();
        let mut query_every = 1u64;
        let mut counter = 0;

        'service_loop: loop {
            counter += 1;
            eprintln!("Records : {:#?}", self.records());

            // // Send query every once in a while
            // let start = Instant::now();
            // while start.elapsed().as_secs() < query_every
            {
                // Sends Query
                self.query()?;
                eprintln!("\n\n[{:>3}] : waiting {} - Querying...", counter, query_every);

                // Listen to incoming UDP packets for 5sec, without sleep between listening
                // connection for 500ms
                if let Some((resp, sender_socket_addr)) = self.recv_timely(1000, 10, query_every)? {
                    if resp.is_query {
                        // if this is a query, we detect conflict and defend our name if we need to
                        if resp.questions.len() > 0 {
                            for question in resp.questions.iter() {
                                match question {
                                    Question::Srv(sda) => {
                                        let q_name = std::str::from_utf8(&sda.name)?;
                                        if self.own_record.hostname() == q_name {
                                            eprintln!(">> Me !");
                                            // Conflict detected, so we defend our name
                                            self.send_socket.send_to(
                                                &resp_packet.to_raw()?,
                                                sender_socket_addr,
                                            )?;
                                        } else {
                                            dbg!("Somewhere else.");
                                            sleep_ms(rand_delay(250, 500));
                                            if let Some(resp_to_query_record) =
                                                self.map_record.get(q_name)
                                            {
                                                let resp_to_query_record_packet =
                                                    resp_to_query_record
                                                        .to_answers_response_packet()
                                                        .to_raw()?;
                                                self.send_socket.send_to(
                                                    &resp_to_query_record_packet,
                                                    &sender_socket_addr,
                                                );
                                            } else {
                                                eprintln!("Oh man, where the hell am I ?!");
                                            }
                                        }
                                    }
                                    Question::Unknown => (),
                                }
                            }
                        }
                    } else {
                        eprintln!(">> Not a Query received.");
                        // Otherwise we update our cache with the info our neighbour just gave us
                        let map_record_from_neighbour = Record::from_packet(&resp);
                        self.update_cache(&map_record_from_neighbour);
                    }
                }
            }

            if query_every < 30 {
                query_every += 1;
            }
            // sleep_ms(query_every * 1000);
            sleep_ms(1000);
        }

        // Ok(())
    }

    /// Builds mDNS probe packet with the proper bit set up in order to check if a host record is
    /// available.
    fn build_probe_packet(&self) -> Packet {
        Packet {
            id: 0x0,
            is_query: true,
            questions: vec![Question::Srv(SrvDataQ {
                name: self.own_record.hostname().as_bytes().to_vec(),
            })],
            answers: vec![],
        }
    }

    /// Builds an mDNS reponse packet containing all the registered ressource records of the host
    /// in the "Answer Section".
    fn build_response_packet(&self, record: &Record) -> Packet {
        self.map_record
            .get(record.hostname())
            .expect("Missing record.")
            .to_answers_response_packet()
    }

    /// Build a packet to defend our name.
    fn build_defensive_packet(&self) -> Packet {
        self.own_record.to_answers_response_packet()
    }

    /// Builds an announcing packet corresponding to an unsolicited mDNS response containing all of
    /// the node's cache.
    fn build_announcing_packet(&self) -> Packet {
        let mut answers = Vec::with_capacity(self.map_record.len());
        for (_name, record) in self.map_record.iter() {
            for ans in record.to_vector_answers().into_iter() {
                answers.push(ans);
            }
        }

        Packet {
            id: 0x0,
            is_query: false,
            questions: vec![],
            answers,
        }
    }

    /// Builds a query packet to be used by one-shot mDNS implementation.
    /// Questions requesting Unicast responses.
    fn build_query_packet(&self) -> Packet {
        let questions = self
            .map_record
            .iter()
            .filter_map(|(name, rec)| {
                if name != self.own_record.hostname() && rec.ttl() > 1 {
                    Some(Question::Srv(SrvDataQ {
                        name: name.as_bytes().to_vec(),
                    }))
                } else {
                    None
                }
            })
            .collect();

        Packet {
            id: 0x0,
            is_query: true,
            questions,
            answers: vec![],
        }
    }

    /// When sending probe queries, a host MUST NOT consult its cache for
    /// potential answers.  Only conflicting Multicast DNS responses received
    /// "live" from the network are considered valid for the purposes of
    /// determining whether probing has succeeded or failed.
    fn probe(&mut self) -> MulticastDnsResult<()> {
        // Making sure our cache is empty before probing
        self.map_record.clear();

        let probe_packet = self.build_probe_packet();

        // Let's wait a moment to give time to other nodes to initialize their network
        let delay_probe_by: u64 = rand_delay(0, 250);
        sleep_ms(delay_probe_by);

        // Abitrary value to prevent any infinite loop if there is some mischief happening on the
        // network
        let mut fail_safe = 0;
        let mut retry = 0;
        while retry < 3 && fail_safe < 1000 {
            fail_safe += 1;
            self.broadcast(&probe_packet)?;
            if let Some((resp, _sender_socket_addr)) = self.recv_timely(750, 0, 500)? {
                let map_record_from_response = Record::from_packet(&resp);
                if let Some(record_from_response) =
                    map_record_from_response.get(self.own_record.hostname())
                {
                    // If confict, then change name and loop back to probe step 1
                    if self.detect_conflict(&record_from_response) {
                        self.resolve_conflict()?;
                        retry = 0;
                    } else {
                        // Let's take authority on our hostname and update our cache accordingly
                        self.map_record
                            .insert(self.own_record.hostname.clone(), self.own_record.clone());
                        break;
                    }
                } else {
                    retry += 1
                }
            } else {
                retry += 1;
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
        Ok(())
    }

    /// Sends unsolicited mDNS responses containing our node's resource records in the "Answer
    /// Section" of a DNS packet.
    fn announcing(&mut self) -> MulticastDnsResult<()> {
        let packet = self.build_announcing_packet();

        // Sends at least 2 time an unsolicited response, up to 8 times maximum
        for _ in 0..rand_delay(2_usize, 9_usize) {
            // Sends mDNS responses containing all of its resource records in the "Answer Section"
            self.broadcast(&packet)?;
            sleep_ms(1_000);
        }
        Ok(())
    }

    /// Detects confict: check if a record already exist in the cache.
    pub fn detect_conflict(&self, record: &Record) -> bool {
        if self.map_record.is_empty() {
            return false;
        } else {
            match self.map_record.get(record.hostname()) {
                Some(_) => return true,
                None => return false,
            }
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

        Ok(confict_free_hostname)
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

fn sleep_ms(duration: u64) {
    thread::sleep(Duration::from_millis(duration));
}

/// Wrapper around randomized range generation.
fn rand_delay<T>(low: T, high: T) -> T
where
    T: rand::distributions::uniform::SampleUniform,
{
    rand::thread_rng().gen_range(low, high)
}

impl Discovery for MulticastDns {
    fn startup(&mut self) {
        for _retry in 0..15 {
            match self.probe() {
                Ok(_) => break,
                Err(ref err) => match err.kind() {
                    error::ErrorKind::ProbeError => {
                        thread::sleep(Duration::from_secs(1));
                    }
                    _ => panic!("Unrecoverable error encountered during probe step."),
                },
            }
        }

        self.announcing().expect("Fail to announce during startup.");
    }

    fn update(&mut self) {
        self.announcing()
            .expect("Fail to update mDNS records during update phase.");
    }

    fn flush(&mut self) {
        self.map_record.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_loop_question() {
        let mut mdns = MulticastDnsBuilder::new()
            .bind_address("0.0.0.0")
            .bind_port(55000)
            .multicast_loop(true)
            .multicast_ttl(255)
            .multicast_address("224.0.0.251")
            .build()
            .expect("build fail");

        let mut packet = dns::Packet::new();
        packet.is_query = true;
        packet.questions.push(dns::Question::Srv(dns::SrvDataQ {
            name: b"lib3h.test.service".to_vec(),
        }));
        mdns.broadcast(&packet).expect("send fail");

        std::thread::sleep(std::time::Duration::from_millis(100));
        let resp = mdns.recv().expect("recv fail");

        match resp.unwrap().questions[0] {
            Question::Srv(ref q) => {
                assert_eq!(b"lib3h.test.service".to_vec(), q.name);
            }
            _ => panic!("BAD TYPE"),
        }
    }

    #[test]
    fn it_should_loop_answer() {
        let mut mdns = MulticastDnsBuilder::new()
            .bind_address("0.0.0.0")
            .bind_port(55001)
            .multicast_loop(true)
            .multicast_ttl(255)
            .multicast_address("224.0.0.251")
            .build()
            .expect("build fail");

        let mut packet = dns::Packet::new();
        packet.id = 0xbdbd;
        packet.is_query = false;
        packet.answers.push(dns::Answer::Srv(dns::SrvDataA {
            name: b"lib3h.test.service".to_vec(),
            ttl_seconds: 0x12345678,
            priority: 0x1111,
            weight: 0x2222,
            port: 0x3333,
            target: b"lib3h.test.target".to_vec(),
        }));
        mdns.broadcast(&packet).expect("send fail");

        std::thread::sleep(std::time::Duration::from_millis(100));
        let resp = mdns.recv().expect("recv fail");

        match resp.unwrap().answers[0] {
            Answer::Srv(ref a) => {
                assert_eq!(b"lib3h.test.service".to_vec(), a.name);
                assert_eq!(0x12345678, a.ttl_seconds);
                assert_eq!(0x1111, a.priority);
                assert_eq!(0x2222, a.weight);
                assert_eq!(0x3333, a.port);
                assert_eq!(b"lib3h.test.target".to_vec(), a.target);
            }
            _ => panic!("BAD TYPE"),
        }
    }

    #[test]
    fn resolve_conflict_test() {
        let mut mdns = MulticastDnsBuilder::new()
            .build()
            .expect("Fail to build mDNS.");

        let own_record = Record::new_own();
        let resolve_confict_name = mdns.resolve_conflict().expect("Fail to resolve confict.");
        assert_ne!(own_record.hostname(), resolve_confict_name);

        let mut mdns = MulticastDnsBuilder::new()
            .own_record(&resolve_confict_name, &["0.0.0.0"])
            .build()
            .expect("Fail to build mDNS.");

        let own_record = Record::new_own();
        let resolve_confict_name = mdns.resolve_conflict().expect("Fail to resolve confict.");
        assert_ne!(own_record.hostname(), resolve_confict_name);

        let mut mdns = MulticastDnsBuilder::new()
            .own_record("asgard.1.2", &["0.0.0.0"])
            .build()
            .expect("Fail to build mDNS.");

        let own_record = Record::new_own();
        let resolve_confict_name = mdns.resolve_conflict().expect("Fail to resolve confict.");
        assert_ne!(own_record.hostname(), resolve_confict_name);
    }
}

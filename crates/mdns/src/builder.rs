//! MulticastDns builder definition.

use crate::{
    error::MulticastDnsError,
    record::{HashMapRecord, MapRecord, Record},
    Instant, MulticastDns, DEFAULT_BIND_ADRESS, DEFAULT_QUERY_INTERVAL_MS, DEFAULT_TTL,
    MDNS_MULCAST_IPV4_ADRESS, READ_BUF_SIZE, SERVICE_LISTENER_PORT,
};

#[cfg(not(target_os = "windows"))]
use net2::unix::UnixUdpBuilderExt;

/// mdns builder
pub struct MulticastDnsBuilder {
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
    /// The amount of time we should wait between two queries.
    pub(crate) query_interval_ms: u128,
    pub(crate) own_map_record: MapRecord,
}

impl MulticastDnsBuilder {
    /// Create a new mDNS builder
    pub fn new() -> Self {
        MulticastDnsBuilder::default()
    }

    /// Create a new mDNS instance with a defined own record.
    pub fn with_own_record(networkid: &str, rec: &Record) -> Self {
        Self {
            own_map_record: MapRecord::with_record(networkid, &[rec.clone()]),
            ..Default::default()
        }
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

    /// Sets the amount of time between two queries originating from ourself.
    pub fn query_interval_ms(&mut self, every_ms: u128) -> &mut Self {
        self.query_interval_ms = every_ms;
        self
    }

    /// Set the host's record.
    pub fn own_record(&mut self, networkid: &str, urls: &[&str]) -> &mut Self {
        let records: Vec<Record> = urls
            .iter()
            .map(|url| Record::new(networkid, url, DEFAULT_TTL))
            .collect();
        self.own_map_record.insert(networkid.to_string(), records);
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

        let send_socket = create_socket(&self.bind_address, self.bind_port)?;
        send_socket.set_nonblocking(true)?;

        Ok(MulticastDns {
            bind_address: self.bind_address.to_owned(),
            bind_port: self.bind_port,
            multicast_loop: self.multicast_loop,
            multicast_ttl: self.multicast_ttl,
            multicast_address: self.multicast_address.to_owned(),
            timestamp: Instant::now(),
            query_interval_ms: self.query_interval_ms,
            send_socket,
            recv_socket,
            buffer: [0; READ_BUF_SIZE],
            own_map_record: self.own_map_record.clone(),
            map_record: MapRecord(HashMapRecord::with_capacity(32)),
        })
    }
}

use std::default::Default;
impl Default for MulticastDnsBuilder {
    fn default() -> Self {
        Self {
            bind_address: String::from(DEFAULT_BIND_ADRESS),
            bind_port: SERVICE_LISTENER_PORT,
            multicast_loop: true,
            multicast_ttl: DEFAULT_TTL,
            multicast_address: String::from(MDNS_MULCAST_IPV4_ADRESS),
            query_interval_ms: DEFAULT_QUERY_INTERVAL_MS,
            own_map_record: MapRecord::new(),
        }
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

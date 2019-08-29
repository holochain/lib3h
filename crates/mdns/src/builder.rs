//! MulticastDns builder definition.

use crate::{
    READ_BUF_SIZE,
    DEFAULT_BIND_ADRESS,
    SERVICE_LISTENER_PORT,
    MDNS_MULCAST_IPV4_ADRESS,
    MulticastDns,
    error::MulticastDnsError,
    record::{Record, MapRecord, HashMapRecord},
};

#[cfg(not(target_os = "windows"))]
use net2::unix::UnixUdpBuilderExt;

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
            &self.bind_address,
            self.bind_port,
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

//! lib3h mDNS LAN discovery module

#![feature(try_trait)]

// use byteorder;
use net2;
use std::net;



#[cfg(not(target_os = "windows"))]
use net2::unix::UnixUdpBuilderExt;

use std::{
    net::ToSocketAddrs, // Used to cast (addr, port) to socket
    collections::HashMap,
};

pub mod error;
pub use error::{MulticastDnsError, MulticastDnsResult};

pub mod dns;
pub use dns::{Answer, Question, Neighbor, Packet};

// 20 byte IP header would mean 65_507... but funky configs can increase that
// const READ_BUF_SIZE: usize = 60_000;
// however... we don't want to accept any packets that big...
// let's stick with one common block size
const READ_BUF_SIZE: usize = 4_096;


/// mdns builder
pub struct MulticastDnsBuilder {
    pub(crate) bind_address: String,
    pub(crate) bind_port: u16,
    pub(crate) multicast_loop: bool,
    pub(crate) multicast_ttl: u32,
    pub(crate) multicast_address: String,
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

    /// construct the actual mdns struct
    pub fn build(&mut self) -> Result<MulticastDns, MulticastDnsError> {
        let socket = create_socket(&self.bind_address, self.bind_port)?;
        socket.set_nonblocking(true)?;
        socket.set_multicast_loop_v4(self.multicast_loop)?;
        socket.set_multicast_ttl_v4(self.multicast_ttl)?;
        socket.join_multicast_v4(
            &self.multicast_address.parse()?,
            &self.bind_address.parse()?,
        )?;

        Ok(MulticastDns {
            bind_address: self.bind_address.to_owned(),
            bind_port: self.bind_port,
            multicast_loop: self.multicast_loop,
            multicast_ttl: self.multicast_ttl,
            multicast_address: self.multicast_address.to_owned(),
            socket,
            buffer: [0; READ_BUF_SIZE],
            neighbors: HashMap::new(),
        })
    }
}

use std::default::Default;
impl Default for MulticastDnsBuilder {
    fn default() -> Self {
        MulticastDnsBuilder {
            bind_address: String::from("0.0.0.0"),
            bind_port: 5353,
            multicast_loop: true,
            multicast_ttl: 255,
            multicast_address: String::from("224.0.0.251"),
        }
    }
}

/// an mdns instance that can send and receive dns packets on LAN UDP multicast
pub struct MulticastDns {
    /// Our IP address bound to UDP Socket, default to `0.0.0.0`
    pub(crate) bind_address: String,
    /// Port used by thge mDNS protocol: `5353`
    pub(crate) bind_port: u16,
    /// If true, multicast packets will be looped back to the local socket
    pub(crate) multicast_loop: bool,
    /// Time to Live: default to `255`
    pub(crate) multicast_ttl: u32,
    /// Multicast address used by the mDNS protocol: `224.0.0.251`
    pub(crate) multicast_address: String,
    /// The socket used by the mDNS service protocol
    socket: net::UdpSocket,
    /// The buffer used to store the packet to send/receive messages
    buffer: [u8; READ_BUF_SIZE],
    /// The lookup table where the neighbors are stored
    neighbors: HashMap<String, Neighbor>,
}

impl MulticastDns {
    /// broadcast a dns packet
    pub fn send(&mut self, packet: &Packet) -> Result<(), MulticastDnsError> {
        let addr = (self.multicast_address.as_ref(), self.bind_port)
            .to_socket_addrs()?
            .next()?;

        let data = packet.to_raw()?;

        self.socket.send_to(&data, &addr)?;

        Ok(())
    }

    /// try to receive a dns packet
    /// will return None rather than blocking if none are queued
    pub fn recv(&mut self) -> Result<Option<Packet>, MulticastDnsError> {
        let (read, _) = match self.socket.recv_from(&mut self.buffer) {
            Ok(r) => r,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    return Ok(None);
                }
                return Err(e.into());
            }
        };

        if read > 0 {
            let packet = Packet::with_raw(&self.buffer[0..read])?;
            Ok(Some(packet))
        } else {
            Ok(None)
        }
    }
}

/// non-windows udp socket bind
#[cfg(not(target_os = "windows"))]
fn create_socket(addr: &str, port: u16) -> Result<std::net::UdpSocket, MulticastDnsError> {
    Ok(net2::UdpBuilder::new_v4()?
        .reuse_address(true)?
        .reuse_port(true)?
        .bind((addr, port))?)
}

/// windows udp socket bind
#[cfg(target_os = "windows")]
fn create_socket(addr: &str, port: u16) -> Result<std::net::UdpSocket, MulticastDnsError> {
    Ok(net2::UdpBuilder::new_v4()?
        .reuse_address(true)?
        .bind((addr, port))?)
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
        mdns.send(&packet).expect("send fail");

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
        mdns.send(&packet).expect("send fail");

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
}

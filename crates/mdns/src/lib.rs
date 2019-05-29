//! lib3h mdns LAN discovery module

#![feature(try_trait)]

extern crate byteorder;
extern crate net2;

// 20 byte IP header would mean 65_507... but funky configs can increase that
// const READ_BUF_SIZE: usize = 60_000;
// however... we don't want to accept any packets that big...
// let's stick with one common block size
const READ_BUF_SIZE: usize = 4_096;

#[cfg(not(target_os = "windows"))]
use net2::unix::UnixUdpBuilderExt;

use std::net::ToSocketAddrs;

pub mod error;
pub use error::{MulticastDnsError, MulticastDnsResult};

pub mod dns;
pub use dns::*;

/// mdns configuration
pub struct Config {
    bind_address: String,
    bind_port: u16,
    multicast_loop: bool,
    multicast_ttl: u32,
    multicast_addr: String,
}

/// mdns builder
pub struct Builder {
    config: Config,
}

impl Builder {
    /// create a new mdns builder
    pub fn new() -> Self {
        Builder {
            config: Config {
                bind_address: "0.0.0.0".to_string(),
                bind_port: 5353,
                multicast_loop: true,
                multicast_ttl: 255,
                multicast_addr: "224.0.0.251".to_string(),
            },
        }
    }

    /// specify the udp port to listen on
    pub fn set_bind_port(mut self, port: u16) -> Self {
        self.config.bind_port = port;
        self
    }

    /// construct the actual mdns struct
    pub fn build(self) -> Result<MulticastDns, MulticastDnsError> {
        MulticastDns::new(self.config)
    }
}

/// an mdns instance that can send and receive dns packets on LAN UDP multicast
pub struct MulticastDns {
    config: Config,
    socket: std::net::UdpSocket,
    read_buf: [u8; READ_BUF_SIZE],
}

impl MulticastDns {
    /// create a new mdns struct instance
    pub fn new(config: Config) -> Result<Self, MulticastDnsError> {
        let socket = create_socket(&config.bind_address, config.bind_port)?;

        socket.set_nonblocking(true)?;
        socket.set_multicast_loop_v4(config.multicast_loop)?;
        socket.set_multicast_ttl_v4(config.multicast_ttl)?;
        socket.join_multicast_v4(
            &config.multicast_addr.parse()?,
            &config.bind_address.parse()?,
        )?;

        Ok(MulticastDns {
            config,
            socket,
            read_buf: [0; READ_BUF_SIZE],
        })
    }

    /// broadcast a dns packet
    pub fn send(&mut self, packet: &Packet) -> Result<(), MulticastDnsError> {
        let addr = (self.config.multicast_addr.as_ref(), self.config.bind_port)
            .to_socket_addrs()?
            .next()?;

        let data = packet.to_raw()?;

        self.socket.send_to(&data, &addr)?;

        Ok(())
    }

    /// try to receive a dns packet
    /// will return None rather than blocking if none are queued
    pub fn recv(&mut self) -> Result<Option<Packet>, MulticastDnsError> {
        let (read, _) = match self.socket.recv_from(&mut self.read_buf) {
            Ok(r) => r,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    return Ok(None);
                }
                return Err(e.into());
            }
        };

        if read > 0 {
            let packet = Packet::with_raw(&self.read_buf[0..read])?;
            return Ok(Some(packet));
        }

        Ok(None)
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
        let mut mdns = Builder::new()
            .set_bind_port(55000)
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
        let mut mdns = Builder::new()
            .set_bind_port(55001)
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

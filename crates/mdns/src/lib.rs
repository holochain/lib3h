//! lib3h mdns LAN discovery module

#![feature(try_trait)]

extern crate byteorder;
extern crate dns_parser;
extern crate net2;

// 20 byte IP header would mean 65_507... but funky configs can increase that
const READ_BUF_SIZE: usize = 60_000;

#[cfg(not(target_os = "windows"))]
use net2::unix::UnixUdpBuilderExt;

use std::net::ToSocketAddrs;

pub mod error;
pub use error::{MulticastDnsError, MulticastDnsResult};

pub mod dns;

pub mod response;
pub use response::Response;

pub struct Config {
    bind_address: String,
    bind_port: u16,
    unicast: bool,
    multicast_loop: bool,
    multicast_ttl: u32,
    multicast_addr: String,
}

pub struct Builder {
    config: Config,
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            config: Config {
                bind_address: "0.0.0.0".to_string(),
                bind_port: 5353,
                unicast: false,
                multicast_loop: true,
                multicast_ttl: 255,
                multicast_addr: "224.0.0.251".to_string(),
            },
        }
    }

    pub fn build(self) -> Result<MulticastDns, MulticastDnsError> {
        MulticastDns::new(self.config)
    }
}

pub struct MulticastDns {
    config: Config,
    socket: std::net::UdpSocket,
    read_buf: [u8; READ_BUF_SIZE],
}

impl MulticastDns {
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

    pub fn send(&mut self) -> Result<(), MulticastDnsError> {
        let addr = (self.config.multicast_addr.as_ref(), self.config.bind_port)
            .to_socket_addrs()?
            .next()?;

        let mut builder = dns_parser::Builder::new_query(0, false);
        builder.add_question(
            "service_name",
            self.config.unicast,
            dns_parser::QueryType::SRV,
            dns_parser::QueryClass::IN,
        );
        let data = builder
            .build()
            .map_err(|_| MulticastDnsError::Generic("Dns Packet Truncated".to_string()))?;

        println!("OTH: {:?}", data);

        /*
        let mut packet = dns::Packet::new();
        packet.questions.push(dns::Question {
            name: b"service_name".to_vec(),
        });
        let data2 = packet.to_raw()?;

        println!("THS: {:?}", data2);

        let mut packet = dns::Packet::new();
        packet.answers.push(dns::Answer {
            name: b"service_name".to_vec(),
        });
        let data2 = packet.to_raw()?;

        println!("THA: {:?}", data2);
        */

        self.socket.send_to(&data, &addr)?;

        Ok(())
    }

    pub fn recv(&mut self) -> Result<Option<Response>, MulticastDnsError> {
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
            let data = dns_parser::Packet::parse(&self.read_buf[0..read])?;
            println!("RAW: {:?}", data);
            return Ok(Some(Response::from_packet(&data)));
        }

        Ok(None)
    }
}

#[cfg(not(target_os = "windows"))]
fn create_socket(addr: &str, port: u16) -> Result<std::net::UdpSocket, MulticastDnsError> {
    Ok(net2::UdpBuilder::new_v4()?
        .reuse_address(true)?
        .reuse_port(true)?
        .bind((addr, port))?)
}

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
    fn it_should_be_sane() {
        let mut mdns = Builder::new().build().expect("build fail");

        mdns.send().expect("send fail");
        mdns.send().expect("send fail");
        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let resp = mdns.recv().expect("recv fail");
            println!("got: {:?}", resp);
        }
    }
}

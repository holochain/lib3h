use http;
use std;

use std::io::{Read, Write};
use std::net::ToSocketAddrs;

pub struct Client {
    pub socket: std::net::TcpStream,
    pub request: http::Request,
}

pub enum State {
    WaitingDidSomething,
    WaitingDidNothing,
    Data,
    Closed,
}

impl Client {
    pub fn new(addr: &str, port: i16) -> Result<Client, std::io::Error> {
        let timeout = std::time::Duration::from_millis(1000);
        let addr = format!("{}:{}", addr, port)
            .to_socket_addrs()?
            .next()
            .unwrap();
        let mut socket = std::net::TcpStream::connect_timeout(&addr, timeout)?;
        socket.set_nonblocking(true)?;

        socket.write(
            r#"GET / HTTP/1.1
Host: www.neonphog.com
User-Agent: funky/743.3
Accept: */*

"#.as_bytes(),
        )?;

        Ok(Client {
            socket: socket,
            request: http::Request::new(http::RequestType::Response),
        })
    }

    pub fn process_once(&mut self) -> Result<State, std::io::Error> {
        let mut buf = [0u8; 1024];

        match self.socket.read(&mut buf) {
            Ok(b) => {
                if b < 1 {
                    // we're done, exit
                    return Ok(State::Closed);
                }
                if self.request.check_parse(&buf[..b]) {
                    return Ok(State::Data);
                }
                return Ok(State::WaitingDidSomething);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return Ok(State::WaitingDidNothing);
            }
            Err(e) => return Err(e),
        }
    }
}

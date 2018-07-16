use errors::*;
use std;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Endpoint {
    pub addr: String,
    pub port: u16,
}

impl Endpoint {
    pub fn new(addr: &str, port: u16) -> Self {
        Endpoint {
            addr: addr.to_string(),
            port: port,
        }
    }

    pub fn to_socket_addr(&self) -> Result<std::net::SocketAddr> {
        let s: String = format!("{}:{}", self.addr, self.port);
        let out: std::net::SocketAddr = s.parse()?;
        Ok(out)
    }
}

impl From<std::net::SocketAddr> for Endpoint {
    fn from(addr: std::net::SocketAddr) -> Self {
        Endpoint {
            addr: match addr {
                std::net::SocketAddr::V4(a) => a.ip().to_string(),
                std::net::SocketAddr::V6(a) => format!("[{}]", a.ip().to_string()),
            },
            port: addr.port(),
        }
    }
}

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.addr, self.port)
    }
}

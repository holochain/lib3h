use http;
use std;

use std::io::Read;

pub struct Client {
    socket: std::net::TcpStream,
    request: http::Request,
}

pub struct Server {
    srv_socket: std::net::TcpListener,
    clients: Vec<Client>,
}

impl Server {
    pub fn new(addr: &str, port: i16) -> Result<Server, std::io::Error> {
        let srv_socket = std::net::TcpListener::bind(format!("{}:{}", addr, port))?;
        srv_socket.set_nonblocking(true)?;

        Ok(Server {
            srv_socket: srv_socket,
            clients: Vec::new(),
        })
    }

    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    pub fn process_once(&mut self) -> Result<bool, std::io::Error> {
        let mut did_something: bool = false;

        // -- first check new connections -- //

        for stream in self.srv_socket.incoming() {
            match stream {
                Ok(s) => {
                    did_something = true;
                    s.set_nonblocking(true)?;
                    let r = http::Request::new(http::RequestType::Request);
                    self.clients.push(Client {
                        socket: s,
                        request: r,
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => return Err(e),
            }
        }

        // -- next check existing connections -- //

        let mut buf = [0u8; 1024];
        let mut new_clients: Vec<Client> = vec![];
        for mut client in self.clients.drain(..) {
            match client.socket.read(&mut buf) {
                Ok(b) => {
                    if b < 1 {
                        // don't add the socket, it's dead
                        continue;
                    }

                    if client.request.check_parse(&buf[..b]) {
                        println!("got: {:?}", client.request);
                    }

                    new_clients.push(client);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    new_clients.push(client);
                }
                Err(e) => return Err(e),
            };
        }
        self.clients = new_clients;

        Ok(did_something)
    }
}

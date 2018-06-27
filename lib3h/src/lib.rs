extern crate byteorder;
extern crate chrono;
extern crate hex;
extern crate inflector;
#[macro_use]
extern crate lazy_static;
extern crate num_bigint;
extern crate num_traits;
extern crate openssl;
extern crate rand;
extern crate regex;
extern crate ring;
extern crate rmp_serde;
extern crate rust_base58;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate tiny_keccak;

pub mod aes;
pub mod client;
pub mod config;
#[macro_use]
pub mod error;
pub mod hash;
pub mod http;
pub mod ident;
pub mod message;
pub mod netinfo;
pub mod node;
pub mod rsa;
pub mod server;
pub mod tcp;
pub mod util;

/*
fn _now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn tcp_test() {
    let mut cli = Some(client::Client::new("neonphog.com", 80).unwrap());
    let mut srv = server::Server::new("0.0.0.0", 8080).unwrap();

    let mut last = _now();
    loop {
        if last != _now() {
            last = _now();
            println!("tcp tick, client count: {}", srv.client_count());
        }
        let mut did_something = false;

        let mut clear_cli = false;
        if let Some(ref mut c) = cli {
            match c.process_once().unwrap() {
                client::State::WaitingDidSomething => {
                    did_something = true;
                }
                client::State::Data => {
                    println!("got: {:?}", c.request);
                    clear_cli = true;
                }
                _ => (),
            }
        }
        if clear_cli {
            cli = None;
        }

        if srv.process_once().unwrap() {
            did_something = true;
        }

        if !did_something {
            // if we didn't process anything, sleep
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

pub fn test() -> String {
    tcp_test();

    String::from("hello")
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
*/

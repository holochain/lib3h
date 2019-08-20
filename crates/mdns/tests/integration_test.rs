// simple p2p mdns usage
extern crate lib3h_mdns;
use lib3h_mdns::{dns_old as dns, MulticastDns, MulticastDnsBuilder};

static SERVICE_NAME: &'static [u8] = b"lib3h.test.service";
static TARGET_NAME: &'static [u8] = b"lib3h.test.target";

struct MdnsResponder {
    mdns: MulticastDns,
}

impl MdnsResponder {
    pub fn new() -> Self {
        let mut out = Self {
            mdns: MulticastDnsBuilder::new().build().unwrap(),
        };

        let mut packet = dns::Packet::new();
        packet.is_query = true;
        packet.questions.push(dns::Question::Srv(dns::SrvDataQ {
            name: SERVICE_NAME.to_vec(),
        }));
        out.mdns.broadcast(&packet).unwrap();

        out
    }

    pub fn process(&mut self) -> Option<Vec<u8>> {
        if let Some(q) = self.mdns.recv().unwrap() {
            if let Some(dns::Question::Srv(s)) = q.0.questions.get(0) {
                if s.name.as_slice() == SERVICE_NAME {
                    // we got a query about our service! send a response
                    let mut packet = dns::Packet::new();
                    packet.id = q.0.id;
                    packet.is_query = false;
                    packet.answers.push(dns::Answer::Srv(dns::SrvDataA {
                        name: SERVICE_NAME.to_vec(),
                        ttl_seconds: 0,
                        priority: 0,
                        weight: 0,
                        port: 0,
                        target: TARGET_NAME.to_vec(),
                    }));
                    self.mdns.broadcast(&packet).expect("send fail");
                }
            }
            if let Some(dns::Answer::Srv(s)) = q.0.answers.get(0) {
                if s.name.as_slice() == SERVICE_NAME {
                    return Some(s.target.clone());
                }
            }
        }

        None
    }
}

#[test]
fn main() {
    let mut r = MdnsResponder::new();
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(r.process(), None);
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(r.process(), Some(TARGET_NAME.to_vec()));
}

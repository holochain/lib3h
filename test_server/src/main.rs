extern crate libsodacon;

use libsodacon::net::node::{Endpoint, Node, StdNetNode};

fn main() {
    let mut node = StdNetNode::new();
    node.listen(&Endpoint::new("127.0.0.1", 8080));

    loop {
        let events = node.process_once();
        if events.len() > 0 {
            for event in events {
                println!("Got event: {:?}", event);
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    println!("Hello, world!");
}

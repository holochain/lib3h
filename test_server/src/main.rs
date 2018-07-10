extern crate libsodacon;

use libsodacon::net::node::{Event, ServerEvent, Endpoint, StdNetNode};

fn main() {
    let mut node = StdNetNode::new(b"hello, me");
    node.listen(&Endpoint::new("127.0.0.1", 8080));
    node.listen(&Endpoint::new("[::1]", 8080));

    loop {
        let events;
        {
            events = node.process_once();
        }

        if events.len() > 0 {
            for event in events {
                println!("Got event: {:?}", event);
                if let Event::OnServerEvent(s) = event {
                    if let ServerEvent::OnListening(endpoint) = s {
                        println!("listening: {}", endpoint);

                        node.connect(&endpoint);
                    }
                }
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

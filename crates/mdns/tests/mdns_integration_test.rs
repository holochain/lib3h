// simple p2p mdns usage
extern crate lib3h_mdns;

use lib3h_mdns::MulticastDnsBuilder;
use lib3h_protocol::discovery::Discovery;

#[test]
fn mdns_integration_test() {
    let url = "wss://127.0.0.1:64159/?a=hc0".to_string();
    // In order to avoid using the service loop MulticastDns::responder()
    // in a separate thread we make our hostname unique
    let hostname = &format!("holonaute::{}::{}", &url, nanoid::simple());
    let addrs = vec![url.as_str()];

    let mut mdns = MulticastDnsBuilder::new()
        .own_record(&hostname, &addrs)
        .build()
        .unwrap();

    mdns.advertise().unwrap();

    let mut records = Vec::new();
    for _ in 0..5 {
        std::thread::sleep(std::time::Duration::from_millis(10));

        for r in mdns.discover().unwrap() {
            records.push(r.clone());
        }
    }

    let mut found = false;
    for r in records.iter() {
        if format!("{}", r) == url {
            found = true;
            break;
        }
    }
    assert!(
        found,
        format!("failed to mdns discover, got {:#?}", records)
    );
}

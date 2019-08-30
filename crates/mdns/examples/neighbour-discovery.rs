use lib3h_discovery::Discovery;
use lib3h_mdns as mdns;
use get_if_addrs;
use std::{thread, time::Duration};

fn list_ip_v4() -> Vec<String> {

       let mut addrs = vec![];
       for iface in get_if_addrs::get_if_addrs().unwrap() {
            if iface.name != "lo" {
                match iface.addr {
                    get_if_addrs::IfAddr::V4(addrv4) => addrs.push(addrv4.ip.to_string()),
                    _ => (),
                }
            }
       }
       addrs
}

fn discover_neighbourhood() {
    let urls = list_ip_v4();
    let urls: Vec<&str> = urls.iter().map(|url| url.as_str()).collect();

    let mut mdns = mdns::MulticastDnsBuilder::new()
        .own_record("holonaute.holo.host", &urls)
        .every(1_000)
        .bind_port(8585)
        .build()
        .expect("Fail to build mDNS.");

    // Make myself known on the network and find a name for ourselves
    println!(">> Advertising...");
    mdns.advertise()
        .expect("Fail to advertise my existence to the world.");

    let mut listen_every_ms = 1_000;
    // Let's listen to the network for a few moments...
    for _ in 0..100 {
        mdns.discover()
            .expect("Fail to discover participants on the network using mDNS.");
        println!(">> mDNS neighbourhood : {:#?}", &mdns.records());

        // Let's wait a few moments before checking if new participants arrived on the network
        thread::sleep(Duration::from_millis(listen_every_ms));

        if listen_every_ms < 30_000 {
            listen_every_ms += 1_000;
        }
    }

    // Let's participants know that we are leaving the network.
    mdns.release()
        .expect("Fail to release ourselves from the network.");
}

fn main() {
    discover_neighbourhood();
}

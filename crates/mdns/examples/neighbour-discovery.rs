use lib3h_discovery::Discovery;
use lib3h_mdns as mdns;

fn discover_neighbourhood() {
    let mut mdns = mdns::MulticastDnsBuilder::new()
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
        mdns::sleep_ms(listen_every_ms);

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

use lib3h_mdns as mdns;
use lib3h_discovery::Discovery;

fn discover_neighbourhood() {
    let mut mdns = mdns::MulticastDnsBuilder::new()
        .bind_port(8585)
        .build()
        .expect("Fail to build mDNS.");

    // Make myself known on the network and find a name for myself
    mdns.advertise()
        .expect("Fail to advertise my existence to the world.");

    // Let's listen to the network for a few moments...
    for _ in 0..100 {
        mdns.discover()
            .expect("Fail to discover participants on the network using mDNS.");
        eprintln!("mDNS neighbourhood : {:#?}", &mdns.records());

        mdns::sleep_ms(5_000);
    }
}

fn main() {
    discover_neighbourhood();
}

use lib3h_mdns as mdns;
use lib3h_mdns::Discovery;

fn discover_neighbourhood() {
    let mut mdns = mdns::MulticastDnsBuilder::new()
        .bind_port(8585)
        .build()
        .expect("Fail to build mDNS.");

    // Make myself known on the network and find a name for myself
    mdns.advertise();
    for _ in 0..100 {
        mdns.discover().expect("Fail to update mDNS.");
        eprintln!("mDNS neighbourhood : {:#?}", &mdns.records());

        mdns::sleep_ms(5_000);
    }
}

fn main() {
    discover_neighbourhood();
}

use lib3h_mdns as mdns;
use lib3h_mdns::Discovery;

fn discover_neighbourhood() {
    let mut mdns = mdns::MulticastDnsBuilder::new()
        .bind_port(8585)
        .build()
        .expect("Fail to build mDNS.");

    // mdns.run().expect("Fail to run mDNS service.");
    mdns.startup();
    for _ in 0..100 {
        mdns.update().expect("Fail to update mDNS.");
        eprintln!("mDNS neighbourhood : {:#?}", &mdns.records());

        mdns::sleep_ms(5_000);
    }

}

fn main() {
    discover_neighbourhood();
}

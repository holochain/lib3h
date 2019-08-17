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
        mdns.update();
        eprintln!("mDNS neighbourhood : {:?}", &mdns.records());

        mdns::sleep_ms(5_000);
    }

}

fn main() {
    let own_record = mdns::record::Record::new_own();

    println!("{:#?}", own_record);

    // let v = 65280;
    // println!("{:#?}", v.as_bytes());

    discover_neighbourhood();
}

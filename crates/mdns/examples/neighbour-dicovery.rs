use lib3h_mdns as mdns;

fn discover_neighbourhood() {
    let mut mdns = mdns::MulticastDnsBuilder::new()
        .bind_port(8585)
        .build()
        .expect("Fail to build mDNS.");

    mdns.run().expect("Fail to run mDNS service.");

    eprintln!("mDNS neighbourhood : {:?}", &mdns.records());
}

fn main() {
    let own_record = mdns::record::Record::new_own();

    println!("{:#?}", own_record);

    // let v = 65280;
    // println!("{:#?}", v.as_bytes());

    discover_neighbourhood();
}

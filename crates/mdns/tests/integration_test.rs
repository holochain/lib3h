// simple p2p mdns usage
extern crate lib3h_mdns;
use lib3h::{
    dht::mirror_dht::MirrorDht,
    engine::{RealEngine, RealEngineConfig},
    transport_wss::TlsConfig,
};
use lib3h_discovery::Discovery;
use lib3h_mdns::MulticastDnsBuilder;
use lib3h_protocol::network_engine::NetworkEngine;
use lib3h_sodium::SodiumCryptoSystem;
use url::Url;

fn basic_setup_wss<'a>() -> RealEngine<'a, MirrorDht> {
    let config = RealEngineConfig {
        tls_config: TlsConfig::Unencrypted,
        socket_type: "ws".into(),
        bootstrap_nodes: vec![],
        work_dir: String::new(),
        log_level: 'd',
        bind_url: Url::parse("wss://127.0.0.1:64519").unwrap(),
        dht_gossip_interval: 200,
        dht_timeout_threshold: 2000,
        dht_custom_config: vec![],
    };
    let engine = RealEngine::new(
        Box::new(SodiumCryptoSystem::new()),
        config,
        "test_engine_wss".into(),
        MirrorDht::new_with_config,
    )
    .unwrap();
    let p2p_binding = engine.advertise();
    println!("test_engine advertise: {}", p2p_binding);
    engine
}

#[test]
fn main() {
    let url = basic_setup_wss().advertise();

    // In order to avoid using the service loop MulticastDns::responder() in a separate thread
    // we make our hostname unique
    let hostname = &format!("holonaute::{}", &url);
    let addrs = vec![url.as_str()];

    let mut mdns = MulticastDnsBuilder::new()
        .own_record(&hostname, &addrs)
        .build()
        .unwrap();

    mdns.advertise().unwrap();

    for _ in 0..5 {
        mdns.discover().unwrap();
        assert_eq!(mdns.records().is_empty(), false);

        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

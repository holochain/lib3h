#[macro_use]
extern crate hexf;
mod utils;
#[macro_use]
extern crate lazy_static;
extern crate backtrace;
extern crate lib3h;
extern crate lib3h_protocol;
extern crate lib3h_sodium;
extern crate lib3h_zombie_actor as lib3h_ghost_actor;
extern crate regex;

#[macro_use]
extern crate log;
use holochain_tracing::test_span;

use lib3h_ghost_actor::{wait1_for_messages, wait_did_work};

use holochain_tracing::Span;
use lib3h::{
    dht::mirror_dht::MirrorDht,
    engine::{EngineConfig, GhostEngine, TransportConfig},
    transport::websocket::tls::TlsConfig,
};

use lib3h_ghost_actor::prelude::*;

use crate::lib3h::engine::CanAdvertise;
use lib3h_protocol::{data_types::*, protocol::*};
use lib3h_sodium::SodiumCryptoSystem;
use std::path::PathBuf;
use url::Url;
use utils::constants::*;
//--------------------------------------------------------------------------------------------------
// Test suites
//--------------------------------------------------------------------------------------------------

//--------------------------------------------------------------------------------------------------
// Logging
//--------------------------------------------------------------------------------------------------

// for this to actually show log entries you also have to run the tests like this:
// RUST_LOG=lib3h=debug cargo test -- --nocapture
fn enable_logging_for_test(enable: bool) {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "trace");
    }
    let _ = env_logger::builder()
        .default_format_timestamp(false)
        .default_format_module_path(false)
        .is_test(enable)
        .try_init();
}

//--------------------------------------------------------------------------------------------------
// Engine Setup
//--------------------------------------------------------------------------------------------------

fn basic_setup_mock_bootstrap<'engine>(
    net: &str,
    name: &str,
    bs: Option<Vec<Url>>,
) -> GhostEngine<'engine> {
    let bootstrap_nodes = match bs {
        Some(s) => s,
        None => vec![],
    };
    let config = EngineConfig {
        transport_configs: vec![TransportConfig::Memory(net.into())],
        bootstrap_nodes,
        work_dir: PathBuf::new(),
        log_level: 'd',
        bind_url: Url::parse(format!("mem://{}", name).as_str()).unwrap(),
        dht_gossip_interval: 100,
        dht_timeout_threshold: 1000,
        dht_custom_config: vec![],
    };
    let engine = GhostEngine::new(
        Span::fixme(),
        Box::new(SodiumCryptoSystem::new()),
        config,
        name.into(),
        MirrorDht::new_with_config,
    )
    .unwrap();
    let p2p_binding = engine.advertise();
    info!(
        "basic_setup_mock(): test engine for {}, advertise: {}",
        name, p2p_binding
    );
    engine
}

fn basic_setup_mock<'engine>(net: &str, name: &str) -> GhostEngine<'engine> {
    basic_setup_mock_bootstrap(net, name, None)
}

fn basic_setup_wss<'engine>(name: &str) -> GhostEngine<'engine> {
    let config = EngineConfig {
        transport_configs: vec![TransportConfig::Websocket(TlsConfig::Unencrypted)],
        bootstrap_nodes: vec![],
        work_dir: PathBuf::new(),
        log_level: 'd',
        bind_url: Url::parse("wss://127.0.0.1:64519").unwrap(),
        dht_gossip_interval: 200,
        dht_timeout_threshold: 2000,
        dht_custom_config: vec![],
    };
    let engine = GhostEngine::new(
        Span::fixme(),
        Box::new(SodiumCryptoSystem::new()),
        config,
        name,
        MirrorDht::new_with_config,
    )
    .unwrap();
    let p2p_binding = engine.advertise();

    info!("test_engine advertise: {}", p2p_binding);
    engine
}

//--------------------------------------------------------------------------------------------------
// Utils
//--------------------------------------------------------------------------------------------------

//--------------------------------------------------------------------------------------------------
// Custom tests
//--------------------------------------------------------------------------------------------------

#[test]
fn basic_track_test_wss() {
    enable_logging_for_test(true);
    // Setup
    let mut engine: GhostEngine = basic_setup_wss("wss_test_node");
    basic_track_test(&mut engine);
}

#[test]
fn basic_track_test_mock() {
    enable_logging_for_test(true);
    // Setup
    let mut engine: GhostEngine = basic_setup_mock("alex", "basic_track_test_mock");
    basic_track_test(&mut engine);
}

fn basic_track_test<'engine>(mut engine: &mut GhostEngine<'engine>) {
    // Test
    let mut track_space = SpaceData {
        request_id: "track_a_1".into(),
        space_address: SPACE_ADDRESS_A.clone(),
        agent_id: ALEX_AGENT_ID.clone(),
    };

    // First track should succeed
    let mut parent_endpoint = engine
        .take_parent_endpoint()
        .unwrap()
        .as_context_endpoint_builder()
        .request_id_prefix("parent")
        .build::<()>();

    parent_endpoint
        .publish(
            test_span("publish join space"),
            ClientToLib3h::JoinSpace(track_space.clone()),
        )
        .unwrap();
    let handle_get_gossip_entry_list_regex =
        "HandleGetGossipingEntryList\\(GetListData \\{ space_address: HashString\\(\"SPACE_A\"\\), provider_agent_id: HashString\\(\"alex\"\\), request_id: \"[\\w\\d_~]*\" \\}\\)";

    let handle_get_authoring_entry_list_regex =
        "HandleGetAuthoringEntryList\\(GetListData \\{ space_address: HashString\\(\"SPACE_A\"\\), provider_agent_id: HashString\\(\"alex\"\\), request_id: \"[\\w\\d_~]*\" \\}\\)";

    let regexes = vec![
        handle_get_authoring_entry_list_regex,
        handle_get_gossip_entry_list_regex,
    ];

    wait1_for_messages!(engine, parent_endpoint, regexes);

    // Track same again, should fail
    track_space.request_id = "track_a_2".into();

    let f: GhostCallback<(), _, _> = Box::new(|&mut _user_data, _cb_data| Ok(()));
    parent_endpoint
        .request(
            test_span("publish join space again"),
            ClientToLib3h::JoinSpace(track_space.clone()),
            f,
        )
        .unwrap();

    wait_did_work!(engine);

    /*
    let handle_failure_result = Box::new(Lib3hServerProtocolEquals(
        Lib3hServerProtocol::FailureResult(GenericResultData {
            request_id: "track_a_2".to_string(),
            space_address: SPACE_ADDRESS_A.clone(),
            to_agent_id: ALEX_AGENT_ID.clone(),
            result_info: "Unknown error encountered: \'Already joined space\'.".into(),
        }),
    ));*/
}

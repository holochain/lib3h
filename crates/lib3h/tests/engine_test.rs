#[macro_use]
extern crate hexf;
#[macro_use]
mod utils;
#[macro_use]
extern crate lazy_static;
extern crate backtrace;
extern crate lib3h;
extern crate lib3h_protocol;
extern crate lib3h_sodium;
extern crate lib3h_zombie_actor as lib3h_ghost_actor;

#[macro_use]
extern crate log;
use lib3h_tracing::test_span;

use lib3h::{
    dht::mirror_dht::MirrorDht,
    engine::{EngineConfig, GhostEngine, TransportConfig},
    transport::websocket::tls::TlsConfig,
};

use lib3h_ghost_actor::prelude::*;

use crate::lib3h::engine::CanAdvertise;
use lib3h_protocol::{data_types::*, protocol::*};
use lib3h_sodium::SodiumCryptoSystem;
use lib3h_tracing::Lib3hSpan;
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

fn basic_setup_mock_bootstrap<'engine>(name: &str, bs: Option<Vec<Url>>) -> GhostEngine<'engine> {
    let bootstrap_nodes = match bs {
        Some(s) => s,
        None => vec![],
    };
    let config = EngineConfig {
        transport_configs: vec![TransportConfig::Memory],
        bootstrap_nodes,
        work_dir: PathBuf::new(),
        log_level: 'd',
        bind_url: Url::parse(format!("mem://{}", name).as_str()).unwrap(),
        dht_gossip_interval: 100,
        dht_timeout_threshold: 1000,
        dht_custom_config: vec![],
    };
    let engine = GhostEngine::new(
        Lib3hSpan::fixme(),
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

fn basic_setup_mock<'engine>(name: &str) -> GhostEngine<'engine> {
    basic_setup_mock_bootstrap(name, None)
}

fn basic_setup_wss(name: &str) -> GhostEngine {
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
        Lib3hSpan::fixme(),
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
    let mut engine: GhostEngine = basic_setup_mock("basic_track_test_mock");
    basic_track_test(&mut engine);
}

fn basic_track_test<'engine>(engine: &mut GhostEngine<'engine>) {
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

    /*
        let is_success_result = Box::new(Lib3hServerProtocolEquals(
            Lib3hServerProtocol::SuccessResult(GenericResultData {
                request_id: "track_a_1".into(),
                space_address: SPACE_ADDRESS_A.clone(),
                to_agent_id: ALEX_AGENT_ID.clone(),
                result_info: vec![].into(),
            }),
        ));

        let handle_get_gosip_entry_list = Box::new(Lib3hServerProtocolAssert(Box::new(
            predicate::function(|x| match x {
                Lib3hServerProtocol::HandleGetGossipingEntryList(_) => true,
                _ => false,
            }),
        )));
        let handle_get_author_entry_list = Box::new(Lib3hServerProtocolAssert(Box::new(
            predicate::function(|x| match x {
                Lib3hServerProtocol::HandleGetAuthoringEntryList(_) => true,
                _ => false,
            }),
        )));

        let processors = vec![
            is_success_result as Box<dyn Processor>,
            handle_get_gosip_entry_list as Box<dyn Processor>,
            handle_get_author_entry_list as Box<dyn Processor>,
        ];
        assert_processed!(engine, engine, processors);
    */
    // Track same again, should fail
    track_space.request_id = "track_a_2".into();

    parent_endpoint
        .publish(
            test_span("publish join space again"),
            ClientToLib3h::JoinSpace(track_space.clone()),
        )
        .unwrap();
    /*
    let handle_failure_result = Box::new(Lib3hServerProtocolEquals(
        Lib3hServerProtocol::FailureResult(GenericResultData {
            request_id: "track_a_2".to_string(),
            space_address: SPACE_ADDRESS_A.clone(),
            to_agent_id: ALEX_AGENT_ID.clone(),
            result_info: "Unknown error encountered: \'Already joined space\'.".into(),
        }),
    ));

    assert_one_processed!(engine, engine, handle_failure_result);

    */
}

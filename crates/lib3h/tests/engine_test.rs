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

use lib3h_ghost_actor::{wait1_for_callback, wait1_for_messages};

use holochain_tracing::{tracer_console::*, Span};
use lib3h::{
    dht::mirror_dht::MirrorDht,
    engine::{EngineConfig, GhostEngine, TransportConfig},
    transport::websocket::tls::TlsConfig,
};

use lib3h_ghost_actor::prelude::*;

use crate::lib3h::{engine::CanAdvertise, LIB3H_TRACER};
use lib3h_protocol::{data_types::*, protocol::*, uri::Lib3hUri};
use lib3h_sodium::SodiumCryptoSystem;
use std::{path::PathBuf, thread, time::Duration};
use url::Url;
use utils::{constants::*, test_network_id};

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
        .default_format_timestamp(true)
        .default_format_module_path(true)
        .is_test(enable)
        .try_init();
}

//--------------------------------------------------------------------------------------------------
// Engine Setup
//--------------------------------------------------------------------------------------------------

fn basic_setup_mock_bootstrap<'engine>(
    net: &str,
    name: &str,
    bs: Option<Vec<Lib3hUri>>,
) -> GhostEngine<'engine> {
    let bootstrap_nodes = match bs {
        Some(s) => s,
        None => vec![],
    };
    let config = EngineConfig {
        network_id: test_network_id(),
        transport_configs: vec![TransportConfig::Memory(net.into())],
        bootstrap_nodes,
        work_dir: PathBuf::new(),
        log_level: 'd',
        bind_url: Lib3hUri::with_memory(name),
        dht_gossip_interval: 100,
        dht_timeout_threshold: 1000,
        dht_custom_config: vec![],
    };
    let root_span: Span = LIB3H_TRACER
        .lock()
        .unwrap()
        .span("(root) basic_setup_mock: GhostEngine::new()")
        .start()
        .into();
    let engine = GhostEngine::new(
        root_span,
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
        network_id: test_network_id(),
        transport_configs: vec![TransportConfig::Websocket(TlsConfig::Unencrypted)],
        bootstrap_nodes: vec![],
        work_dir: PathBuf::new(),
        log_level: 'd',
        bind_url: Url::parse("wss://127.0.0.1:64519").unwrap().into(),
        dht_gossip_interval: 200,
        dht_timeout_threshold: 2000,
        dht_custom_config: vec![],
    };
    let root_span: Span = LIB3H_TRACER
        .lock()
        .unwrap()
        .span("(root) basic_setup_mock: GhostEngine::new()")
        .start()
        .into();
    let engine = GhostEngine::new(
        root_span,
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
// Custom tests
//--------------------------------------------------------------------------------------------------

#[test]
fn basic_track_test_wss() {
    enable_logging_for_test(true);
    // Create tracer & reporter
    let (tracer, mut reporter) = new_tracer_with_console_reporter();
    {
        let mut singleton = LIB3H_TRACER.lock().unwrap();
        *singleton = tracer;
    }
    // Setup
    let mut engine: GhostEngine = basic_setup_wss("wss_test_node");
    // Launch
    basic_track_test(&mut engine);
    // Print spans
    let count = reporter.drain();
    println!("span count = {}", count);
    reporter.print(false);
}

#[test]
fn basic_track_test_mock() {
    enable_logging_for_test(true);
    // Create tracer & reporter
    let (tracer, mut reporter) = new_tracer_with_console_reporter();
    {
        let mut singleton = LIB3H_TRACER.lock().unwrap();
        *singleton = tracer;
    }
    // Setup
    let mut engine: GhostEngine = basic_setup_mock("alex", "basic_track_test_mock");
    // Launch
    basic_track_test(&mut engine);
    // Print spans
    let count = reporter.drain();
    println!("span count = {}", count);
    reporter.print(false);
}

fn basic_track_test<'engine>(mut engine: &mut GhostEngine<'engine>) {
    let mut root_span: Span = LIB3H_TRACER
        .lock()
        .unwrap()
        .span("(root) basic_track_test")
        .start()
        .into();
    root_span.event("start");
    // SystemTime is not monotonic so wait a bit to make sure following spans are shown after this span
    thread::sleep(Duration::from_millis(1));

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
        .build::<Option<String>>();

    parent_endpoint
        .publish(
            root_span.child("send event ClientToLib3h::JoinSpace"),
            ClientToLib3h::JoinSpace(track_space.clone()),
        )
        .unwrap();
    let handle_get_gossip_entry_list_regex =
            "HandleGetGossipingEntryList\\(GetListData \\{ space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), provider_agent_id: AgentPubKey\\(HashString\\(\"alex\"\\)\\), request_id: \"[\\w\\d_~]*\" \\}\\)";

    let handle_get_authoring_entry_list_regex =
            "HandleGetAuthoringEntryList\\(GetListData \\{ space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), provider_agent_id: AgentPubKey\\(HashString\\(\"alex\"\\)\\), request_id: \"[\\w\\d_~]*\" \\}\\)";

    let regexes = vec![
        handle_get_authoring_entry_list_regex,
        handle_get_gossip_entry_list_regex,
    ];

    wait1_for_messages!(engine, parent_endpoint, None, regexes);

    // Track same again, should fail
    track_space.request_id = "track_a_2".into();
    let expected = "Response(Err(Lib3hError(Other(\"Already joined space\"))))";

    wait1_for_callback!(
        engine,
        parent_endpoint,
        ClientToLib3h::JoinSpace(track_space),
        expected
    );
}

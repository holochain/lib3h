#[macro_use]
mod utils;
#[macro_use]
extern crate lazy_static;
extern crate backtrace;
extern crate lib3h;
extern crate lib3h_protocol;
extern crate lib3h_sodium;
extern crate predicates;

use predicates::prelude::*;

use lib3h::{
    dht::mirror_dht::MirrorDht,
    engine::{RealEngine, RealEngineConfig},
};
use lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol,
};
use lib3h_sodium::SodiumCryptoSystem;
use url::Url;
use utils::{
    constants::*,
    processor_harness::{Lib3hServerProtocolAssert, Lib3hServerProtocolEquals, Processor},
};

//--------------------------------------------------------------------------------------------------
// Test suites
//--------------------------------------------------------------------------------------------------

type TwoEnginesTestFn = fn(alex: &mut Box<dyn NetworkEngine>, billy: &mut Box<dyn NetworkEngine>);

lazy_static! {
    pub static ref TWO_ENGINES_BASIC_TEST_FNS: Vec<(TwoEnginesTestFn, bool)> = vec![
        (setup_only, true),
        (basic_two_send_message, true),
        (basic_two_join_first, false),
    ];
}

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

fn basic_setup_mock_bootstrap(name: &str, bs: Option<Vec<Url>>) -> RealEngine {
    let bootstrap_nodes = match bs {
        Some(s) => s,
        None => vec![],
    };
    let config = RealEngineConfig {
        // tls_config: TlsConfig::Unencrypted,
        socket_type: "mem".into(),
        bootstrap_nodes,
        work_dir: String::new(),
        log_level: 'd',
        bind_url: Url::parse(format!("mem://{}", name).as_str()).unwrap(),
        dht_gossip_interval: 100,
        dht_timeout_threshold: 1000,
        dht_custom_config: vec![],
    };
    let engine = RealEngine::new_mock(
        Box::new(SodiumCryptoSystem::new()),
        config,
        name.into(),
        MirrorDht::new_with_config,
    )
    .unwrap();
    let p2p_binding = engine.advertise();
    println!(
        "basic_setup_mock(): test engine for {}, advertise: {}",
        name, p2p_binding
    );
    engine
}

fn basic_setup_mock(name: &str) -> RealEngine {
    basic_setup_mock_bootstrap(name, None)
}

// FIXME
//fn basic_setup_wss() -> RealEngine {
//    let config = RealEngineConfig {
//        // tls_config: TlsConfig::Unencrypted,
//        socket_type: "ws".into(),
//        bootstrap_nodes: vec![],
//        work_dir: String::new(),
//        log_level: 'd',
//        bind_url: Url::parse("wss://127.0.0.1:64519").unwrap(),
//        dht_gossip_interval: 200,
//        dht_timeout_threshold: 2000,
//        dht_custom_config: vec![],
//    };
//    let engine = RealEngine::new(
//        Box::new(SodiumCryptoSystem::new()),
//        config,
//        "test_engine_wss".into(),
//        MirrorDht::new_with_config,
//    )
//    .unwrap();
//    let p2p_binding = engine.advertise();
//    println!("test_engine advertise: {}", p2p_binding);
//    engine
//}

//--------------------------------------------------------------------------------------------------
// Utils
//--------------------------------------------------------------------------------------------------

fn print_two_engines_test_name(print_str: &str, test_fn: TwoEnginesTestFn) {
    print_test_name(print_str, test_fn as *mut std::os::raw::c_void);
}

/// Print name of test function
fn print_test_name(print_str: &str, test_fn: *mut std::os::raw::c_void) {
    backtrace::resolve(test_fn, |symbol| {
        let mut full_name = symbol.name().unwrap().as_str().unwrap().to_string();
        let mut test_name = full_name.split_off("engine_test::".to_string().len());
        test_name.push_str("()");
        println!("{}{}", print_str, test_name);
    });
}

//--------------------------------------------------------------------------------------------------
// Custom tests
//--------------------------------------------------------------------------------------------------

#[test]
#[ignore]
fn basic_connect_test_mock() {
    enable_logging_for_test(true);
    // Setup
    let mut engine_a: Box<dyn NetworkEngine> =
        Box::new(basic_setup_mock("basic_send_test_mock_node_a"));
    let mut engine_b: Box<dyn NetworkEngine> =
        Box::new(basic_setup_mock("basic_send_test_mock_node_b"));
    // Get URL
    let url_b = engine_b.advertise();
    println!("url_b: {}", url_b);
    // Send Connect Command
    let request_id: String = "".into();
    let connect_msg = ConnectData {
        request_id: request_id.clone(),
        peer_uri: url_b.clone(),
        network_id: NETWORK_A_ID.clone(),
    };
    engine_a
        .post(Lib3hClientProtocol::Connect(connect_msg.clone()))
        .unwrap();
    println!("\nengine_a.process()...");

    wait_connect!(engine_a, connect_msg, engine_b);
}

#[test]
#[ignore]
fn basic_connect_bootstrap_test_mock() {
    enable_logging_for_test(true);
    // Setup
    let engine_b = basic_setup_mock("basic_connect_bootstrap_test_node_b");
    // Get URL
    let url_b = engine_b.advertise();
    println!("url_b: {}", url_b);

    // Create a with b as a bootstrap
    let mut engine_a =
        basic_setup_mock_bootstrap("basic_connect_bootstrap_test_node_a", Some(vec![url_b]));

    println!("\nengine_a.process()...");
    let (did_work, srv_msg_list) = engine_a.process().unwrap();
    println!("engine_a: {:?}", srv_msg_list);
    assert!(did_work);
}

// FIXME
//#[test]
//fn basic_track_test_wss() {
//    enable_logging_for_test(true);
//    // Setup
//    let mut engine: Box<dyn NetworkEngine> = Box::new(basic_setup_wss());
//    basic_track_test(&mut engine);
//}

#[test]
#[ignore]
fn basic_track_test_mock() {
    enable_logging_for_test(true);
    // Setup
    let mut engine: Box<dyn NetworkEngine> = Box::new(basic_setup_mock("basic_track_test_mock"));
    basic_track_test(&mut engine);
}

fn basic_track_test(engine: &mut Box<dyn NetworkEngine>) {
    // Test
    let mut track_space = SpaceData {
        request_id: "track_a_1".into(),
        space_address: SPACE_ADDRESS_A.clone(),
        agent_id: ALEX_AGENT_ID.clone(),
    };
    // First track should succeed
    engine
        .post(Lib3hClientProtocol::JoinSpace(track_space.clone()))
        .unwrap();

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

    // Track same again, should fail
    track_space.request_id = "track_a_2".into();

    engine
        .post(Lib3hClientProtocol::JoinSpace(track_space.clone()))
        .unwrap();

    let handle_failure_result = Box::new(Lib3hServerProtocolEquals(
        Lib3hServerProtocol::FailureResult(GenericResultData {
            request_id: "track_a_2".to_string(),
            space_address: SPACE_ADDRESS_A.clone(),
            to_agent_id: ALEX_AGENT_ID.clone(),
            result_info: "Already joined space".into(),
        }),
    ));

    assert_one_processed!(engine, engine, handle_failure_result);
}

#[test]
#[ignore]
fn basic_two_nodes_mock() {
    enable_logging_for_test(true);
    // Launch tests on each setup
    for (test_fn, can_setup) in TWO_ENGINES_BASIC_TEST_FNS.iter() {
        launch_two_nodes_test_with_memory_network(*test_fn, *can_setup).unwrap();
    }
}

// Do general test with config
fn launch_two_nodes_test_with_memory_network(
    test_fn: TwoEnginesTestFn,
    can_setup: bool,
) -> Result<(), ()> {
    println!("");
    print_two_engines_test_name("IN-MEMORY TWO ENGINES TEST: ", test_fn);
    println!("=======================");

    // Setup
    let mut alex: Box<dyn NetworkEngine> = Box::new(basic_setup_mock("alex"));
    let mut billy: Box<dyn NetworkEngine> = Box::new(basic_setup_mock("billy"));
    if can_setup {
        basic_two_setup(&mut alex, &mut billy);
    }

    // Execute test
    test_fn(&mut alex, &mut billy);

    // Wrap-up test
    println!("==================");
    print_two_engines_test_name("IN-MEMORY TWO ENGINES TEST END: ", test_fn);

    // Done
    Ok(())
}

/// Empty function that triggers the test suite
fn setup_only(_alex: &mut Box<dyn NetworkEngine>, _billy: &mut Box<dyn NetworkEngine>) {
    // n/a
}

///
fn basic_two_setup(alex: &mut Box<dyn NetworkEngine>, billy: &mut Box<dyn NetworkEngine>) {
    // Connect Alex to Billy
    let req_connect = ConnectData {
        // TODO Should be able to specify a non blank string
        request_id: "".to_string(),
        peer_uri: billy.advertise(),
        network_id: NETWORK_A_ID.clone(),
    };

    alex.post(Lib3hClientProtocol::Connect(req_connect.clone()))
        .unwrap();

    wait_connect!(alex, req_connect, billy);

    // Alex joins space A
    println!("\n Alex joins space \n");
    let mut track_space = SpaceData {
        request_id: "track_a_1".into(),
        space_address: SPACE_ADDRESS_A.clone(),
        agent_id: ALEX_AGENT_ID.clone(),
    };

    track_space.agent_id = ALEX_AGENT_ID.clone();
    alex.post(Lib3hClientProtocol::JoinSpace(track_space.clone()))
        .unwrap();
    track_space.agent_id = BILLY_AGENT_ID.clone();
    billy
        .post(Lib3hClientProtocol::JoinSpace(track_space.clone()))
        .unwrap();

    // TODO check for join space response messages.

    wait_did_work!(alex, billy);
    wait_until_no_work!(alex, billy);

    println!("DONE basic_two_setup DONE \n\n\n");
}

//
fn basic_two_send_message(alex: &mut Box<dyn NetworkEngine>, billy: &mut Box<dyn NetworkEngine>) {
    // Create message
    let req_dm = DirectMessageData {
        space_address: SPACE_ADDRESS_A.clone(),
        request_id: "dm_1".to_string(),
        to_agent_id: BILLY_AGENT_ID.clone(),
        from_agent_id: ALEX_AGENT_ID.clone(),
        content: "wah".into(),
    };
    // Send
    println!("\nAlex sends DM to Billy...\n");
    alex.post(Lib3hClientProtocol::SendDirectMessage(req_dm.clone()))
        .unwrap();
    let is_success_result = Box::new(Lib3hServerProtocolEquals(
        Lib3hServerProtocol::SuccessResult(GenericResultData {
            request_id: req_dm.clone().request_id,
            space_address: SPACE_ADDRESS_A.clone(),
            to_agent_id: ALEX_AGENT_ID.clone(),
            result_info: "".into(),
        }),
    ));

    let handle_send_direct_message = Box::new(Lib3hServerProtocolEquals(
        Lib3hServerProtocol::HandleSendDirectMessage(req_dm.clone()),
    ));

    let processors = vec![is_success_result, handle_send_direct_message];
    // Send / Receive request
    assert_processed!(alex, billy, processors);

    // Post response
    let mut res_dm = req_dm.clone();
    res_dm.to_agent_id = req_dm.from_agent_id.clone();
    res_dm.from_agent_id = req_dm.to_agent_id.clone();
    res_dm.content = format!("echo: {}", req_dm.content).as_bytes().into();
    billy
        .post(Lib3hClientProtocol::HandleSendDirectMessageResult(
            res_dm.clone(),
        ))
        .unwrap();

    let is_success_result = Box::new(Lib3hServerProtocolEquals(
        Lib3hServerProtocol::SuccessResult(GenericResultData {
            request_id: res_dm.clone().request_id,
            space_address: SPACE_ADDRESS_A.clone(),
            to_agent_id: BILLY_AGENT_ID.clone(),
            result_info: "".into(),
        }),
    ));

    let handle_send_direct_message_result = Box::new(Lib3hServerProtocolEquals(
        Lib3hServerProtocol::SendDirectMessageResult(res_dm.clone()),
    ));

    let processors = vec![is_success_result, handle_send_direct_message_result];

    assert_processed!(alex, billy, processors);
}

//
fn basic_two_join_first(alex: &mut Box<dyn NetworkEngine>, billy: &mut Box<dyn NetworkEngine>) {
    // Setup: Track before connecting

    // A joins space
    let mut track_space = SpaceData {
        request_id: "track_a_1".into(),
        space_address: SPACE_ADDRESS_A.clone(),
        agent_id: ALEX_AGENT_ID.clone(),
    };
    println!("\n Alex joins space \n");
    alex.post(Lib3hClientProtocol::JoinSpace(track_space.clone()))
        .unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();

    // Billy joins space
    println!("\n Billy joins space \n");
    track_space.agent_id = BILLY_AGENT_ID.clone();
    billy
        .post(Lib3hClientProtocol::JoinSpace(track_space.clone()))
        .unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();

    // Connect Alex to Billy
    let req_connect = ConnectData {
        // TODO Should be able to set a non blank request id
        request_id: "".to_string(),
        peer_uri: billy.advertise(),
        network_id: NETWORK_A_ID.clone(),
    };
    println!("\n Alex connects to Billy \n");
    alex.post(Lib3hClientProtocol::Connect(req_connect.clone()))
        .unwrap();

    wait_connect!(alex, req_connect, billy);

    println!("DONE Setup for basic_two_multi_join() DONE \n\n\n");

    wait_until_no_work!(alex, billy);

    // Do Send DM test
    basic_two_send_message(alex, billy);
}

#[macro_use]
mod utils;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate unwrap_to;
extern crate backtrace;
extern crate lib3h;
extern crate lib3h_protocol;
extern crate lib3h_sodium;
extern crate predicates;

use predicates::prelude::*;

use lib3h::{
    dht::{dht_trait::Dht, mirror_dht::MirrorDht},
    engine::{RealEngine, RealEngineConfig},
    transport_wss::TlsConfig,
};
use lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol,
};
use lib3h_sodium::SodiumCryptoSystem;
use url::Url;
use utils::constants::*;

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

fn basic_setup_mock(name: &str) -> RealEngine<MirrorDht> {
    let config = RealEngineConfig {
        tls_config: TlsConfig::Unencrypted,
        socket_type: "mem".into(),
        bootstrap_nodes: vec![],
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

#[derive(Clone, Debug)]
struct ProcessorArgs {
    did_work: bool,
    engine_name: String,
    events: Vec<Lib3hServerProtocol>,
    previous: Vec<ProcessorArgs>,
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub enum ProcessorResult {
    Continue(String),
    Fail(String),
    Pass,
}

/// A function that produces accepted sockets of type R wrapped in a TransportInfo
type Processor = Box<dyn FnMut(ProcessorArgs) -> ProcessorResult>;

const MAX_PROCESSING_LOOPS: u64 = 20;

pub fn assert_using_predicate<'a>(
    mut engines: &mut Vec<&mut RealEngine<'a, MirrorDht>>,
    predicate: Box<dyn Predicate<Lib3hServerProtocol>>,
    expect: String,
) {
    let mut f: Processor = Box::new(move |processor_args: ProcessorArgs| {
        if processor_args
            .events
            .iter()
            .find(|e| predicate.eval(e))
            .is_some()
        {
            ProcessorResult::Pass
        } else {
            ProcessorResult::Continue(format!("Expected: {:?}", expect.clone()))
        }
    });
    let actual = process_until(&mut engines, &mut f);
    assert_eq!(ProcessorResult::Pass, actual)
}

fn process_until<'a>(engines: &mut Vec<&mut RealEngine<'a, MirrorDht>>, f: &mut Processor) -> ProcessorResult {
    let mut previous = Vec::new();
    let mut errors = Vec::new();
    for epoch in 0..MAX_PROCESSING_LOOPS {
        println!("[{:?}] {:?}", epoch, previous);
        for engine in engines.iter_mut() {
            let (did_work, events) = engine
                .process()
                .map_err(|err| dbg!(err))
                .unwrap_or((false, vec![]));
            let events = dbg!(events);
            let processor_args = ProcessorArgs {
                did_work,
                events,
                engine_name: engine.name(),
                previous: previous.clone(),
            };
            let result = (f)(processor_args.clone());
            match result {
                ProcessorResult::Continue(err) => {
                    errors.push(err);
                    ()
                }
                ProcessorResult::Fail(err) => return ProcessorResult::Fail(err),
                ProcessorResult::Pass => return ProcessorResult::Pass,
            };
            if processor_args.did_work {
                previous.push(processor_args.clone());
            }
        }
    }
    ProcessorResult::Fail(format!(
        "Max number of process iterations exceeded ({}). Errors: {:?}",
        MAX_PROCESSING_LOOPS,
        errors
    ))
}

fn is_connected(x: &Lib3hServerProtocol, request_id: &str) -> bool {
    match x {
        Lib3hServerProtocol::Connected(data) => data.request_id == request_id,
        _ => false,
    }
}

#[test]
fn basic_connect_test_mock() {
    enable_logging_for_test(true);
    // Setup
    let mut engine_a = basic_setup_mock("basic_send_test_mock_node_a");
    let mut engine_b = basic_setup_mock("basic_send_test_mock_node_b");
    // Get URL
    let url_b = engine_b.advertise();
    println!("url_b: {}", url_b);
    // Send Connect Command
    let request_id: String = "connect_a_1".into();
    let connect_msg = ConnectData {
        request_id: request_id.clone(),
        peer_uri: url_b.clone(),
        network_id: NETWORK_A_ID.clone(),
    };
    engine_a
        .post(Lib3hClientProtocol::Connect(connect_msg.clone()))
        .unwrap();
    println!("\nengine_a.process()...");
    // TODO request_id should match on "connect_a_1" not ""
    let is_connected = Box::new(predicate::function(|x| is_connected(x, "")));

    let mut engines = vec![&mut engine_a, &mut engine_b];

    assert_using_predicate(&mut engines, is_connected, "Connected".into())
}

#[test]
fn basic_track_test_wss() {
    enable_logging_for_test(true);
    // Setup
    let mut engine = basic_setup_wss();
    basic_track_test(&mut engine);
}

#[test]
fn basic_track_test_mock() {
    enable_logging_for_test(true);
    // Setup
    let mut engine = basic_setup_mock("basic_track_test_mock");
    basic_track_test(&mut engine);
}

fn basic_track_test<D: Dht>(engine: &mut RealEngine<D>) {
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
    let (did_work, srv_msg_list) = engine.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 3);
    let res_msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::SuccessResult);
    assert_eq!(res_msg.request_id, "track_a_1".to_string());
    assert_eq!(res_msg.space_address, *SPACE_ADDRESS_A);
    assert_eq!(res_msg.to_agent_id, *ALEX_AGENT_ID);
    println!(
        "SuccessResult info: {}",
        std::str::from_utf8(res_msg.result_info.as_slice()).unwrap()
    );
    let _ = unwrap_to!(srv_msg_list[1] => Lib3hServerProtocol::HandleGetGossipingEntryList);
    let _ = unwrap_to!(srv_msg_list[2] => Lib3hServerProtocol::HandleGetAuthoringEntryList);
    // Track same again, should fail
    track_space.request_id = "track_a_2".into();
    engine
        .post(Lib3hClientProtocol::JoinSpace(track_space))
        .unwrap();
    let (did_work, srv_msg_list) = engine.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let res_msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::FailureResult);
    assert_eq!(res_msg.request_id, "track_a_2".to_string());
    assert_eq!(res_msg.space_address, *SPACE_ADDRESS_A);
    assert_eq!(res_msg.to_agent_id, *ALEX_AGENT_ID);
    println!(
        "FailureResult info: {}",
        std::str::from_utf8(res_msg.result_info.as_slice()).unwrap()
    );
}

#[test]
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
        request_id: "connect".to_string(),
        peer_uri: billy.advertise(),
        network_id: NETWORK_A_ID.clone(),
    };
    alex.post(Lib3hClientProtocol::Connect(req_connect.clone()))
        .unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let connected_msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::Connected);
    println!("connected_msg = {:?}", connected_msg);
    assert_eq!(connected_msg.uri, req_connect.peer_uri);
    // More process: Have Billy process P2p::PeerAddress of alex
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();

    // Alex joins space A
    println!("\n Alex joins space \n");
    let mut track_space = SpaceData {
        request_id: "track_a_1".into(),
        space_address: SPACE_ADDRESS_A.clone(),
        agent_id: ALEX_AGENT_ID.clone(),
    };
    alex.post(Lib3hClientProtocol::JoinSpace(track_space.clone()))
        .unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    // More process
    let (_did_work, _srv_msg_list) = billy.process().unwrap();

    // Billy joins space A
    println!("\n Billy joins space \n");
    track_space.agent_id = BILLY_AGENT_ID.clone();
    billy
        .post(Lib3hClientProtocol::JoinSpace(track_space.clone()))
        .unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    // More process
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();

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
        content: "wah".as_bytes().to_vec(),
    };
    // Send
    println!("\nAlex sends DM to Billy...\n");
    alex.post(Lib3hClientProtocol::SendDirectMessage(req_dm.clone()))
        .unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::SuccessResult(response) = msg_1 {
        assert_eq!(response.request_id, req_dm.request_id);
    });
    // Receive
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::HandleSendDirectMessage);
    assert_eq!(msg, &req_dm);
    let content = std::str::from_utf8(msg.content.as_slice()).unwrap();
    println!("HandleSendDirectMessage: {}", content);

    // Post response
    let mut res_dm = req_dm.clone();
    res_dm.to_agent_id = req_dm.from_agent_id.clone();
    res_dm.from_agent_id = req_dm.to_agent_id.clone();
    res_dm.content = format!("echo: {}", content).as_bytes().to_vec();
    billy
        .post(Lib3hClientProtocol::HandleSendDirectMessageResult(
            res_dm.clone(),
        ))
        .unwrap();
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::SuccessResult(response) = msg_1 {
        assert_eq!(response.request_id, res_dm.request_id);
    });
    // Receive response
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::SendDirectMessageResult);
    assert_eq!(msg, &res_dm);
    let content = std::str::from_utf8(msg.content.as_slice()).unwrap();
    println!("SendDirectMessageResult: {}", content);
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
        request_id: "connect".to_string(),
        peer_uri: billy.advertise(),
        network_id: NETWORK_A_ID.clone(),
    };
    println!("\n Alex connects to Billy \n");
    alex.post(Lib3hClientProtocol::Connect(req_connect.clone()))
        .unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let connected_msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::Connected);
    println!("connected_msg = {:?}", connected_msg);
    assert_eq!(connected_msg.uri, req_connect.peer_uri);
    // More process: Have Billy process P2p::PeerAddress of alex
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();

    println!("DONE Setup for basic_two_multi_join() DONE \n\n\n");

    // Do Send DM test
    basic_two_send_message(alex, billy);
}

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
    dht::mirror_dht::MirrorDht,
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

trait Processor: Predicate<ProcessorArgs> + PartialEq + std::fmt::Display {
    fn name(&self) -> String {
        "default_processor".into()
    }

    fn test(&self, args : &ProcessorArgs);
}

trait AssertEquals<T: PartialEq + std::fmt::Debug> : Processor {
    fn actual(&self, args: &ProcessorArgs) -> Option<T>;

    fn expected(&self) -> T;

    fn eval(&self, args: &ProcessorArgs) -> bool {
        if let Some(actual) = self.actual(args) {
            actual == self.expected()
        } else {
            false
        }
    }

    fn test(&self, args: &ProcessorArgs) {
        assert_eq!(Some(self.expected()), self.actual(args));
    }

    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }

}

#[derive(PartialEq, Debug)]
struct Lib3hServerProtocolEquals(Lib3hServerProtocol);

impl AssertEquals<Lib3hServerProtocol> for Lib3hServerProtocolEquals {
    fn actual(&self, args: &ProcessorArgs) -> Option<Lib3hServerProtocol> {
        let x = args.events.first().clone();
        x.map(|x| x.clone())
    }
    fn expected(&self) -> Lib3hServerProtocol {
        self.0.clone()
    }
}

impl std::fmt::Display for Lib3hServerProtocolEquals {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl predicates::Predicate<ProcessorArgs> for Lib3hServerProtocolEquals {
    fn eval(&self, args: &ProcessorArgs) -> bool {
        if let Some(actual) = self.actual(args) {
            actual == self.expected()
        } else {
            false
        }
    }
}

impl Processor for Lib3hServerProtocolEquals {
    fn test(&self, args: &ProcessorArgs) {
        assert_eq!(Some(self.expected()), self.actual(args));
    }
}

impl predicates::reflection::PredicateReflection for Lib3hServerProtocolEquals {}


const MAX_PROCESSING_LOOPS: u64 = 20;

fn assert_processed(
    engines: &mut Vec<&mut Box<dyn NetworkEngine>>,
    processors: &Vec<Box<impl Processor>>,
) {
    let mut previous = Vec::new();
    let mut errors = Vec::new();

    for p in processors {
        errors.push((p, None))
    }

    for epoch in 0..MAX_PROCESSING_LOOPS {
        println!("[{:?}] {:?}", epoch, previous);

        for engine in engines.iter_mut() {
            let (did_work, events) = engine
                .process()
                .map_err(|err| dbg!(err))
                .unwrap_or((false, vec![]));
            if events.is_empty()
            {
                continue;
            }

            let events = dbg!(events);
            let processor_args = ProcessorArgs {
                did_work,
                events,
                engine_name: engine.name(),
                previous: previous.clone(),
            };
            let mut failed2 = Vec::new();

            for (processor, _orig_processor_args) in errors.drain(..) {
                let result = processor.eval(&processor_args.clone());
                if result {
                    processor.test(&processor_args.clone());
                // processor passed!
                } else {
                    failed2.push((processor, Some(processor_args.clone())));
                }
            }
            errors.append(&mut failed2);
            if !processor_args.events.is_empty() {
                previous.push(processor_args.clone());
            }

            if errors.is_empty() {
                return; 
            }
        }
    }

    for (p, args) in errors {
        if let Some(args) = args {
            p.test(&args)
        } else {
            panic!(format!("Never tested predicate: TODO"))
        }
    }
}

fn is_connected(request_id: &str, uri: url::Url) -> Lib3hServerProtocolEquals {
    Lib3hServerProtocolEquals(Lib3hServerProtocol::Connected(ConnectedData {
        request_id: request_id.into(),
        uri,
    }))
}

fn _is_success_result(x: &Lib3hServerProtocol, expected: GenericResultData) -> bool {
    match x {
        Lib3hServerProtocol::SuccessResult(actual) => expected == *actual,
        _ => false,
    }
}

#[test]
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
    //let is_connected = Box::new(predicate::function(|x| is_connected(x, "")));

    let is_connected = Box::new(is_connected("", url::Url::parse("foo://bar").unwrap())); // engine_a.advertise()));

    let mut engines = vec![&mut engine_a, &mut engine_b];

    assert_processed(&mut engines, &vec![is_connected]);
}

#[test]
fn basic_track_test_wss() {
    enable_logging_for_test(true);
    // Setup
    let mut engine: Box<dyn NetworkEngine> = Box::new(basic_setup_wss());
    basic_track_test(&mut engine);
}

#[test]
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
    let (did_work, srv_msg_list) = engine.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 3);

    let mut engines = vec![engine];

    /*   let is_success_result =
            Box::new(predicate::function(|x|
             is_success_result(x, GenericResultData {
                request_id:"track_a_1".into(),
                space_address:SPACE_ADDRESS_A.clone(),
                to_agent_id:ALEX_AGENT_ID.clone(),
                result_info : vec![].into()})));

        assert_using_predicate(&mut engines, is_success_result, "Track Space Success Result".into());
    */
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

    let mut engines2 = Vec::new();
    for engine in engines.drain(..) {
        engine
            .post(Lib3hClientProtocol::JoinSpace(track_space.clone()))
            .unwrap();
        engines2.push(engine);
    }

    /*   let (did_work, srv_msg_list) = engine.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let res_msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::FailureResult);
    assert_eq!(res_msg.request_id, "track_a_2".to_string());
    assert_eq!(res_msg.space_address, *SPACE_ADDRESS_A);
    assert_eq!(res_msg.to_agent_id, *ALEX_AGENT_ID);
    println!(
        "FailureResult info: {}",
        std::str::from_utf8(res_msg.result_info.as_slice()).unwrap()
    );*/
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
    //    assert_using_predicate(alex

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

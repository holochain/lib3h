#[macro_use]
mod utils;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate unwrap_to;
extern crate backtrace;
#[macro_use]
extern crate log;
extern crate holochain_persistence_api;
#[macro_use]
extern crate failure;
extern crate lib3h;
extern crate lib3h_protocol;
extern crate multihash;

mod node_mock;

use lib3h::{
    dht::mirror_dht::MirrorDht,
    engine::{RealEngine, RealEngineConfig},
    transport::memory_mock::transport_memory::TransportMemory,
    transport_wss::TlsConfig,
};
use lib3h_crypto_api::{FakeCryptoSystem, InsecureBuffer};
use lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_server::Lib3hServerProtocol, Address,
    Lib3hResult,
};
use node_mock::NodeMock;
use rmp_serde::Deserializer;
use serde::Deserialize;
use url::Url;
use utils::constants::*;

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
// Engine factories
//--------------------------------------------------------------------------------------------------

fn construct_mock_engine(
    config: &RealEngineConfig,
    name: &str,
) -> Lib3hResult<Box<dyn NetworkEngine>> {
    let engine: RealEngine<TransportMemory, MirrorDht, InsecureBuffer, FakeCryptoSystem> =
        RealEngine::new_mock(config.clone(), name.into(), MirrorDht::new_with_config).unwrap();
    let p2p_binding = engine.advertise();
    println!(
        "construct_mock_engine(): test engine for {}, advertise: {}",
        name, p2p_binding
    );
    Ok(Box::new(engine))
}

//--------------------------------------------------------------------------------------------------
// Node Setup
//--------------------------------------------------------------------------------------------------

pub type NodeFactory = fn(name: &str, agent_id_arg: Address) -> NodeMock;

fn setup_memory_node(name: &str, agent_id_arg: Address) -> NodeMock {
    let config = RealEngineConfig {
        tls_config: TlsConfig::Unencrypted,
        socket_type: "mem".into(),
        bootstrap_nodes: vec![],
        work_dir: String::new(),
        log_level: 'd',
        bind_url: Url::parse(format!("mem://{}", name).as_str()).unwrap(),
        dht_custom_config: vec![],
    };
    NodeMock::new_with_config(name, agent_id_arg, config, construct_mock_engine)
}

#[allow(dead_code)]
fn setup_wss_node(name: &str, agent_id_arg: Address) -> NodeMock {
    let config = RealEngineConfig {
        tls_config: TlsConfig::Unencrypted,
        socket_type: "ws".into(),
        bootstrap_nodes: vec![],
        work_dir: String::new(),
        log_level: 'd',
        bind_url: Url::parse("wss://127.0.0.1:64519").unwrap(),
        dht_custom_config: vec![],
    };
    NodeMock::new_with_config(name, agent_id_arg, config, construct_mock_engine)
}

//--------------------------------------------------------------------------------------------------
// Utils
//--------------------------------------------------------------------------------------------------

fn print_two_nodes_test_name(print_str: &str, test_fn: TwoNodesTestFn) {
    print_test_name(print_str, test_fn as *mut std::os::raw::c_void);
}

/// Print name of test function
fn print_test_name(print_str: &str, test_fn: *mut std::os::raw::c_void) {
    backtrace::resolve(test_fn, |symbol| {
        let mut full_name = symbol.name().unwrap().as_str().unwrap().to_string();
        let mut test_name = full_name.split_off("integration_test::".to_string().len());
        test_name.push_str("()");
        println!("{}{}", print_str, test_name);
    });
}

//--------------------------------------------------------------------------------------------------
// Test Suites
//--------------------------------------------------------------------------------------------------

type TwoNodesTestFn = fn(alex: &mut NodeMock, billy: &mut NodeMock);

lazy_static! {
    pub static ref TWO_NODES_BASIC_TEST_FNS: Vec<(TwoNodesTestFn, bool)> = vec![
        //(setup_only, true),
        //(two_nodes_send_message, true),
        (two_nodes_dht_publish_test, true),
        //(two_nodes_dht_hold_test, true),
    ];
}

//--------------------------------------------------------------------------------------------------
// Test launchers
//--------------------------------------------------------------------------------------------------

#[test]
fn test_two_memory_nodes_suite() {
    enable_logging_for_test(true);
    for (test_fn, can_setup) in TWO_NODES_BASIC_TEST_FNS.iter() {
        launch_two_memory_nodes_test(*test_fn, *can_setup).unwrap();
    }
}

// Do general test with config
fn launch_two_memory_nodes_test(test_fn: TwoNodesTestFn, can_setup: bool) -> Result<(), ()> {
    println!("");
    print_two_nodes_test_name("IN-MEMORY TWO NODES TEST: ", test_fn);
    println!("=======================");

    // Setup
    let mut alex = setup_memory_node("alex", ALEX_AGENT_ID.clone());
    let mut billy = setup_memory_node("billy", BILLY_AGENT_ID.clone());
    if can_setup {
        setup_two_nodes(&mut alex, &mut billy);
    }

    // Execute test
    test_fn(&mut alex, &mut billy);

    // Wrap-up test
    println!("========================");
    print_two_nodes_test_name("IN-MEMORY TWO NODES TEST END: ", test_fn);
    // Terminate nodes
    alex.stop();
    billy.stop();

    Ok(())
}

//--------------------------------------------------------------------------------------------------
// Test setup
//--------------------------------------------------------------------------------------------------

///
fn setup_two_nodes(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Start
    alex.run();
    billy.run();

    // Connect Alex to Billy
    alex.connect_to(&billy.advertise()).unwrap();

    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let connected_msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::Connected);
    println!("connected_msg = {:?}", connected_msg);
    assert_eq!(&connected_msg.uri, &billy.advertise());
    // More process: Have Billy process P2p::PeerAddress of alex
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();

    // Alex joins space A
    alex.join_space(&SPACE_ADDRESS_A.clone(), true).unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();

    // Billy joins space A
    billy.join_space(&SPACE_ADDRESS_A.clone(), true).unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();

    let (_did_work, _srv_msg_list) = billy.process().unwrap();

    println!("DONE setup_two_nodes() DONE \n\n\n");
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

/// Empty function that triggers the test suite
fn setup_only(_alex: &mut NodeMock, _billy: &mut NodeMock) {
    // n/a
}

/// Test SendDirectMessage and response
fn two_nodes_send_message(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Send DM
    let req_id = alex.send_direct_message(&BILLY_AGENT_ID, "wah".as_bytes().to_vec());
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 0);
    // Receive
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::HandleSendDirectMessage);
    assert_eq!(msg.request_id, req_id);
    let content = std::str::from_utf8(msg.content.as_slice()).unwrap();
    println!("HandleSendDirectMessage: {}", content);

    // Send response
    let response_content = format!("echo: {}", content).as_bytes().to_vec();
    billy.send_response(&req_id, &alex.agent_id, response_content.clone());
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 0);
    // Receive response
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::SendDirectMessageResult);
    let content = std::str::from_utf8(msg.content.as_slice()).unwrap();
    println!("SendDirectMessageResult: {}", content);
    assert_eq!(msg.content, response_content);
}

/// Test publish, Store, Query
fn two_nodes_dht_publish_test(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Alex publish data on the network
    alex.author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
        .unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 0);

    // #fullsync
    // Alex or Billy should receive the entry store request
    let store_result = billy.wait(Box::new(one_is!(
        Lib3hServerProtocol::HandleStoreEntryAspect(_)
    )));
    assert!(store_result.is_some());
    println!("\n got HandleStoreEntryAspect: {:?}", store_result);
    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
    let (did_work, _srv_msg_list) = billy.process().unwrap();
    assert!(did_work);

    // Billy asks for that entry
    // ==========================
    println!("\nBilly requesting entry: ENTRY_ADDRESS_1\n");
    let query_data = billy.request_entry(ENTRY_ADDRESS_1.clone());
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 0);

    println!("\nBilly reply to own request:\n");

    // #fullsync
    // Billy sends that data back to the network
    let _ = billy.reply_to_HandleQueryEntry(&query_data).unwrap();
    let (did_work, srv_msg_list) = billy.process().unwrap();
    println!("\nBilly gets own response {:?}\n", srv_msg_list);
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::QueryEntryResult);
    assert_eq!(&msg.entry_address, &*ENTRY_ADDRESS_1);
    let mut de = Deserializer::new(&msg.query_result[..]);
    let maybe_entry: Result<EntryData, rmp_serde::decode::Error> =
        Deserialize::deserialize(&mut de);
    assert_eq!(
        &maybe_entry.unwrap().aspect_list[0].aspect,
        &*ASPECT_CONTENT_1
    );

    // Billy asks for unknown entry
    // ============================
    let query_data = billy.request_entry(ENTRY_ADDRESS_2.clone());
    let res = alex.reply_to_HandleQueryEntry(&query_data);
    println!("\nAlex gives response {:?}\n", res);
    assert!(res.is_err());
    let res_data: GenericResultData = res.err().unwrap();
    let res_info = std::str::from_utf8(res_data.result_info.as_slice()).unwrap();
    assert_eq!(res_info, "No entry found");
}

/// Test Hold & Query
fn two_nodes_dht_hold_test(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Alex holds an entry
    alex.hold_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
        .unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);

    // #fullsync
    // mirrorDht wants the entry to broadcast it
    assert_eq!(srv_msg_list.len(), 1);
    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::HandleFetchEntry);
    assert_eq!(&msg.entry_address, &*ENTRY_ADDRESS_1);
    alex.reply_to_HandleFetchEntry(msg).unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 0);
    // Process the HoldEntry generated from receiving HandleStoreEntryAspect
    let (did_work, _srv_msg_list) = billy.process().unwrap();
    assert!(did_work);

    // Billy asks for that entry
    // =========================
    println!("\nBilly requesting entry: ENTRY_ADDRESS_1\n");
    let query_data = billy.request_entry(ENTRY_ADDRESS_1.clone());
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 0);

    // #fullsync
    // Billy sends that data back to the network
    println!("\nBilly reply to own request\n",);
    let _ = billy.reply_to_HandleQueryEntry(&query_data).unwrap();
    let (did_work, srv_msg_list) = billy.process().unwrap();
    println!("\nBilly gets own response {:?}\n", srv_msg_list);
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::QueryEntryResult);
    assert_eq!(&msg.entry_address, &*ENTRY_ADDRESS_1);
    let mut de = Deserializer::new(&msg.query_result[..]);
    let maybe_entry: Result<EntryData, rmp_serde::decode::Error> =
        Deserialize::deserialize(&mut de);
    assert_eq!(
        &maybe_entry.unwrap().aspect_list[0].aspect,
        &*ASPECT_CONTENT_1
    );

    // Billy asks for unknown entry
    // ============================
    println!("\nBilly requesting unknown entry:\n");
    let query_data = billy.request_entry(ENTRY_ADDRESS_2.clone());
    let res = alex.reply_to_HandleQueryEntry(&query_data);
    println!("\nAlex gives response {:?}\n", res);
    assert!(res.is_err());
    let res_data: GenericResultData = res.err().unwrap();
    let res_info = std::str::from_utf8(res_data.result_info.as_slice()).unwrap();
    assert_eq!(res_info, "No entry found");
}

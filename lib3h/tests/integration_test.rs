extern crate holochain_lib3h;
extern crate lib3h_protocol;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate unwrap_to;

use lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol, Address,
};
// #[cfg(test)]
use holochain_lib3h::{
    real_engine::{RealEngine, RealEngineConfig},
    transport::{memory_mock::transport_memory::TransportMemory, transport_trait::Transport},
    transport_wss::TransportWss,
};

//--------------------------------------------------------------------------------------------------
// Constants
//--------------------------------------------------------------------------------------------------

lazy_static! {
    pub static ref NETWORK_A_ID: String = "net_A".to_string();
    pub static ref ALEX_AGENT_ID: Address = "alex".to_string().into_bytes();
    pub static ref BILLY_AGENT_ID: Address = "billy".to_string().into_bytes();
    pub static ref SPACE_ADDRESS_A: Address = "SPACE_A".to_string().into_bytes();
}

//--------------------------------------------------------------------------------------------------
// Setup
//--------------------------------------------------------------------------------------------------

fn basic_setup_mock(name: &str) -> RealEngine<TransportMemory> {
    let config = RealEngineConfig {
        socket_type: "ws".into(),
        bootstrap_nodes: vec![],
        work_dir: String::new(),
        log_level: 'd',
    };
    let engine = RealEngine::new_mock(config, name.into()).unwrap();
    let p2p_binding = engine.advertise();
    println!("test_engine advertise: {}", p2p_binding);
    engine
}

fn basic_setup_wss() -> RealEngine<TransportWss<std::net::TcpStream>> {
    let config = RealEngineConfig {
        socket_type: "ws".into(),
        bootstrap_nodes: vec![],
        work_dir: String::new(),
        log_level: 'd',
    };
    let engine = RealEngine::new(config, "test_engine_wss".into()).unwrap();
    let p2p_binding = engine.advertise();
    println!("test_engine advertise: {}", p2p_binding);
    engine
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[test]
fn basic_connect_test_mock() {
    // Setup
    let mut engine_a = basic_setup_mock("basic_send_test_mock_node_a");
    let mut engine_b = basic_setup_mock("basic_send_test_mock_node_b");
    engine_a.run().unwrap();
    engine_b.run().unwrap();
    // Get URL
    let url_b = engine_b.advertise();
    println!("url_b: {}", url_b);
    // Send Connect Command
    let mut connect_msg = ConnectData {
        request_id: "connect_a_1".into(),
        peer_transport: url_b.clone(),
        network_id: NETWORK_A_ID.clone(),
    };
    engine_a
        .post(Lib3hClientProtocol::Connect(connect_msg.clone()))
        .unwrap();
    println!("\nengine_a.process()...");
    let (did_work, srv_msg_list) = engine_a.process().unwrap();
    println!("engine_a: {:?}", srv_msg_list);
    assert!(did_work);
    engine_a.terminate().unwrap();
    engine_b.terminate().unwrap();
}

#[test]
fn basic_track_test_wss() {
    // Setup
    let mut engine = basic_setup_wss();
    basic_track_test(&mut engine);
}

#[test]
fn basic_track_test_mock() {
    // Setup
    let mut engine = basic_setup_mock("basic_track_test_mock");
    basic_track_test(&mut engine);
}

fn basic_track_test<T: Transport>(engine: &mut RealEngine<T>) {
    // Start
    engine.run().unwrap();

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
    assert_eq!(srv_msg_list.len(), 1);
    let res_msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::SuccessResult);
    assert_eq!(res_msg.request_id, "track_a_1".to_string());
    assert_eq!(res_msg.space_address, SPACE_ADDRESS_A.as_slice());
    assert_eq!(res_msg.to_agent_id, ALEX_AGENT_ID.as_slice());
    println!(
        "SuccessResult info: {}",
        std::str::from_utf8(res_msg.result_info.as_slice()).unwrap()
    );
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
    assert_eq!(res_msg.space_address, SPACE_ADDRESS_A.as_slice());
    assert_eq!(res_msg.to_agent_id, ALEX_AGENT_ID.as_slice());
    println!(
        "FailureResult info: {}",
        std::str::from_utf8(res_msg.result_info.as_slice()).unwrap()
    );
    // Done
    engine.terminate().unwrap();
}

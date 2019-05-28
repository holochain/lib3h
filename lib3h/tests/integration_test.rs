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

use holochain_lib3h::real_engine::{RealEngine, RealEngineConfig};

//--------------------------------------------------------------------------------------------------
// Constants
//--------------------------------------------------------------------------------------------------

lazy_static! {
    pub static ref ALEX_AGENT_ID: Address = "alex".to_string().into_bytes();
    pub static ref SPACE_ADDRESS_A: Address = "SPACE_A".to_string().into_bytes();
}

//--------------------------------------------------------------------------------------------------
// Setup
//--------------------------------------------------------------------------------------------------

fn basic_setup() -> RealEngine {
    let config = RealEngineConfig {
        socket_type: "ws".into(),
        bootstrap_nodes: vec![],
        work_dir: String::new(),
        log_level: 'd',
    };
    let engine = RealEngine::new(config, "test_engine".into()).unwrap();
    let p2p_binding = engine.advertise();
    println!("test_engine advertise: {}", p2p_binding);
    engine
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[test]
fn basic_track_test() {
    // Setup
    let mut engine = basic_setup();
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

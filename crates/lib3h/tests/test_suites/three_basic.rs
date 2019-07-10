use crate::{node_mock::NodeMock, utils::constants::*};
use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol, Address};
use rmp_serde::Deserializer;
use serde::Deserialize;

pub type ThreeNodesTestFn = fn(alex: &mut NodeMock, billy: &mut NodeMock, camille: &mut NodeMock);

lazy_static! {
    pub static ref THREE_NODES_BASIC_TEST_FNS: Vec<(ThreeNodesTestFn, bool)> = vec![
        // (test_setup_only, true),
        (test_send_message, true),
//        (test_author_and_hold, true),
    ];
}

//--------------------------------------------------------------------------------------------------
// Test setup
//--------------------------------------------------------------------------------------------------

///
pub fn setup_three_nodes(alex: &mut NodeMock, billy: &mut NodeMock, camille: &mut NodeMock) {
    // Start
    alex.run();
    billy.run();
    camille.run();

    // Connection
    // ==========
    // Connect Alex to Billy
    alex.connect_to(&billy.advertise()).unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let connected_msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::Connected);
    println!("[Alex] connected_msg = {:?}\n", connected_msg);
    assert_eq!(&connected_msg.uri, &billy.advertise());
    // More process: Have Billy process P2p::PeerAddress of alex
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();

    // Connect Camille to Billy
    camille.connect_to(&billy.advertise()).unwrap();
    let (did_work, srv_msg_list) = camille.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let connected_msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::Connected);
    println!("[Camille] connected_msg = {:?}\n", connected_msg);
    assert_eq!(&connected_msg.uri, &billy.advertise());
    // More process: Have Billy process P2p::PeerAddress of Camille
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = camille.process().unwrap();

    // Extra processing required for auto-handshaking
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = camille.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();

    let (_did_work, _srv_msg_list) = camille.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();

    let (_did_work, _srv_msg_list) = camille.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();

    // Space joining
    // =============
    // Alex joins space
    println!("\n Alex joins space \n");
    let req_id = alex.join_space(&SPACE_ADDRESS_A, true).unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 3);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::SuccessResult(response) = msg_1 {
        assert_eq!(response.request_id, req_id);
    });
    // Extra processing required for auto-handshaking
    let (_did_work, _srv_msg_list) = billy.process().unwrap();

    // Billy joins space
    println!("\n Billy joins space \n");
    let req_id = billy.join_space(&SPACE_ADDRESS_A, true).unwrap();
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 3);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::SuccessResult(response) = msg_1 {
        assert_eq!(response.request_id, req_id);
    });
    // Extra processing required for auto-handshaking
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = camille.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();

    // Camille joins space
    println!("\n Camille joins space \n");
    let req_id = camille.join_space(&SPACE_ADDRESS_A, true).unwrap();
    let (did_work, srv_msg_list) = camille.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 3);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::SuccessResult(response) = msg_1 {
        assert_eq!(response.request_id, req_id);
    });
    // Extra processing required for auto-handshaking
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = camille.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = camille.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();

    println!("DONE setup_three_nodes() DONE \n\n\n");
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

/// Empty function that triggers the test suite
fn test_setup_only(_alex: &mut NodeMock, _billy: &mut NodeMock, _camille: &mut NodeMock) {
    // n/a
}

/// Test SendDirectMessage and response
fn test_send_message(alex: &mut NodeMock, billy: &mut NodeMock, camille: &mut NodeMock) {
//    // A sends DM to B
//    // ===============
//    let req_id = alex.send_direct_message(&BILLY_AGENT_ID, "wah".as_bytes().to_vec());
//    assert_process_success!(alex, req_id);
//    // B should receive
//    let (did_work, srv_msg_list) = billy.process().unwrap();
//    assert!(did_work);
//    assert_eq!(srv_msg_list.len(), 1);
//    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::HandleSendDirectMessage);
//    assert_eq!(msg.request_id, req_id);
//    let content = std::str::from_utf8(msg.content.as_slice()).unwrap();
//    println!("HandleSendDirectMessage: {}", content);
//    // C should not receive
//    let (did_work, srv_msg_list) = camille.process().unwrap();
//    assert!(!did_work);
//    assert_eq!(srv_msg_list.len(), 0);
//
//    // Send response
//    println!("\nBilly responds to Alex...\n");
//    let response_content = format!("echo: {}", content).as_bytes().to_vec();
//    billy.send_response(&req_id, &alex.agent_id, response_content.clone());
//    assert_process_success!(billy, req_id);
//    // A receives response
//    let (did_work, srv_msg_list) = alex.process().unwrap();
//    assert!(did_work);
//    assert_eq!(srv_msg_list.len(), 1);
//    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::SendDirectMessageResult);
//    let content = std::str::from_utf8(msg.content.as_slice()).unwrap();
//    println!("SendDirectMessageResult: {}", content);
//    assert_eq!(msg.content, response_content);
//    // C should not receive
//    let (did_work, srv_msg_list) = camille.process().unwrap();
//    assert!(!did_work);
//    assert_eq!(srv_msg_list.len(), 0);

    // C sends DM to A
    // ===============
    println!("\nCamille sends DM to Alex...\n");
    let req_id = camille.send_direct_message(&ALEX_AGENT_ID, "marco".as_bytes().to_vec());
//    let (did_work, srv_msg_list) = camille.process().unwrap();
//    println!("response({}): {:?}", did_work, srv_msg_list);
    assert_process_success!(camille, req_id);
    // A should receive
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::HandleSendDirectMessage);
    assert_eq!(msg.request_id, req_id);
    let content = std::str::from_utf8(msg.content.as_slice()).unwrap();
    println!("HandleSendDirectMessage: {}", content);
    // B should not receive
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(!did_work);
    assert_eq!(srv_msg_list.len(), 0);

    // Send response
    println!("\nAlex responds to Camille...\n");
    let response_content = format!("echo: {}", content).as_bytes().to_vec();
    alex.send_response(&req_id, &camille.agent_id, response_content.clone());
    assert_process_success!(alex, req_id);
    // Receive response
    let (did_work, srv_msg_list) = camille.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::SendDirectMessageResult);
    let content = std::str::from_utf8(msg.content.as_slice()).unwrap();
    println!("SendDirectMessageResult: {}", content);
    assert_eq!(msg.content, response_content);
    // B should not receive
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(!did_work);
    assert_eq!(srv_msg_list.len(), 0);
}

/// Test publish, Store, Query
fn test_author_and_hold(alex: &mut NodeMock, billy: &mut NodeMock, camille: &mut NodeMock) {
    // FIXME
}

use crate::{node_mock::NodeMock, utils::constants::*};
use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol, Lib3hResult};
use rmp_serde::Deserializer;
use serde::Deserialize;

pub type TwoNodesTestFn = fn(alex: &mut NodeMock, billy: &mut NodeMock);

lazy_static! {
    pub static ref TWO_NODES_BASIC_TEST_FNS: Vec<(TwoNodesTestFn, bool)> = vec![
        (setup_only, true),
        (two_nodes_send_message, true),
        (two_nodes_dht_publish_test, true),
        (two_nodes_dht_hold_test, true),
    ];
}

//--------------------------------------------------------------------------------------------------
// Test setup
//--------------------------------------------------------------------------------------------------

///
pub fn setup_two_nodes(alex: &mut NodeMock, billy: &mut NodeMock) {
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
// Helpers
//--------------------------------------------------------------------------------------------------

/// Request ENTRY_ADDRESS_1 from the network and should get it back
pub fn request_entry_1(node: &mut NodeMock) {
    println!("\n{} requesting entry: ENTRY_ADDRESS_1\n", node.name);
    let query_data = node.request_entry(ENTRY_ADDRESS_1.clone());
    let (did_work, _srv_msg_list) = node.process().unwrap();
    assert!(did_work);

    // #fullsync
    // Billy sends that data back to the network
    println!("\n{} reply to own request:\n", node.name);
    let _ = node.reply_to_HandleQueryEntry(&query_data).unwrap();
    let (did_work, srv_msg_list) = node.process().unwrap();
    println!("\n{} gets own response {:?}\n", node.name, srv_msg_list);
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
    request_entry_1(billy);

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
    request_entry_1(billy);

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

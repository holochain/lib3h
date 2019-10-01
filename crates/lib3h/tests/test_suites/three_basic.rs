use crate::{
    node_mock::{test_join_space, NodeMock},
    utils::constants::*,
};
use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol};
use rmp_serde::Deserializer;
use serde::Deserialize;

pub type ThreeNodesTestFn = fn(alex: &mut NodeMock, billy: &mut NodeMock, camille: &mut NodeMock);

lazy_static! {
    pub static ref THREE_NODES_BASIC_TEST_FNS: Vec<(ThreeNodesTestFn, bool)> = vec![
        (test_setup_only, true),
        (test_send_message, true),
        // TODO will uncomment as they are fixed

//        (test_author_and_hold, true),
    ];
}

//--------------------------------------------------------------------------------------------------
// Test setup
//--------------------------------------------------------------------------------------------------

///
pub fn setup_three_nodes(
    /*mut*/ alex: &mut NodeMock,
    billy: &mut NodeMock,
    /*mut*/ camille: &mut NodeMock,
) {
    // Connection
    // ==========
    // Connect Alex to Billy
    let connect_data = alex.connect_to(&billy.advertise()).unwrap();
    wait_connect!(alex, connect_data, billy);

    billy.wait_until_no_work();
    alex.wait_until_no_work();
    billy.wait_until_no_work();

    // Connect Camille to Billy
    let connect_data = camille.connect_to(&billy.advertise()).unwrap();
    wait_connect!(camille, connect_data, billy);
    // More process: Have Billy process P2p::PeerName of Camille
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = camille.process().unwrap();
    // More process so Camille can handshake with billy
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = camille.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();

    // Space joining
    // =============
    // Alex joins space
    test_join_space(alex, &SPACE_ADDRESS_A);
    // Extra processing required for auto-handshaking
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = camille.process().unwrap();

    // Billy joins space
    test_join_space(billy, &SPACE_ADDRESS_A);
    // Extra processing required for auto-handshaking
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = camille.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();

    // Camille joins space
    test_join_space(camille, &SPACE_ADDRESS_A);
    // Extra processing required for auto-handshaking
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = camille.process().unwrap();
    let (_did_work, _srv_msg_list) = camille.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();

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
    // A sends DM to B
    // ===============
    let _req_id = alex.send_direct_message(&BILLY_AGENT_ID, "wah".as_bytes().to_vec());

    // B should receive
    let expected = "HandleSendDirectMessage\\(DirectMessageData \\{ space_address: HashString\\(\"appA\"\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"billy\"\\), from_agent_id: HashString\\(\"alex\"\\), content: \"wah\" \\}\\)";
    let results = assert2_msg_matches!(alex, billy, expected);

    // C should not receive
    let (_did_work, srv_msg_list) = camille.process().unwrap();
    //   assert!(!did_work);  FIXME?
    assert_eq!(srv_msg_list.len(), 0);

    // billy sends reponse to alex so we need to get the message for its request_id
    let handle_send_direct_msg = results.first().unwrap();
    let event = handle_send_direct_msg.events.first().unwrap();
    let msg = unwrap_to!(event => Lib3hServerProtocol::HandleSendDirectMessage);
    let response_content = format!("echo: {}", "wah").as_bytes().to_vec();
    billy.send_response(&msg.request_id, &alex.agent_id(), response_content.clone());
    let expected = "SendDirectMessageResult\\(DirectMessageData \\{ space_address: HashString\\(\"appA\"\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"alex\"\\), from_agent_id: HashString\\(\"billy\"\\), content: \"echo: wah\" \\}\\)";
    assert2_msg_matches!(alex, billy, expected);

    // C should not receive
    let (_did_work, srv_msg_list) = camille.process().unwrap();
    //   assert!(!did_work);  FIXME?
    assert_eq!(srv_msg_list.len(), 0);

    // C sends DM to A
    // ===============
    let _req_id = camille.send_direct_message(&ALEX_AGENT_ID, "marco".as_bytes().to_vec());
    // X should receive
    let expected = "HandleSendDirectMessage\\(DirectMessageData \\{ space_address: HashString\\(\"appA\"\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"alex\"\\), from_agent_id: HashString\\(\"camille\"\\), content: \"marco\" \\}\\)";
    let results = assert2_msg_matches!(alex, camille, expected);

    // B should not receive
    let (_did_work, srv_msg_list) = billy.process().unwrap();
    //assert!(!did_work);
    assert_eq!(srv_msg_list.len(), 0);

    // alex sends reponse to camille so we need to get the message for its request_id
    let handle_send_direct_msg = results.first().unwrap();
    let event = handle_send_direct_msg.events.first().unwrap();
    let msg = unwrap_to!(event => Lib3hServerProtocol::HandleSendDirectMessage);
    let response_content = format!("echo: {}", "marco").as_bytes().to_vec();
    alex.send_response(
        &msg.request_id,
        &camille.agent_id(),
        response_content.clone(),
    );
    let expected = "SendDirectMessageResult\\(DirectMessageData \\{ space_address: HashString\\(\"appA\"\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"camille\"\\), from_agent_id: HashString\\(\"alex\"\\), content: \"echo: marco\" \\}\\)";
    assert2_msg_matches!(alex, camille, expected);

    // B should not receive
    let (_did_work, srv_msg_list) = billy.process().unwrap();
    //assert!(!did_work);
    assert_eq!(srv_msg_list.len(), 0);

    // alex sends a response again and req_id should be expired
    let result = alex.send_response_inner(
        &msg.request_id,
        &camille.agent_id(),
        response_content.clone(),
    );
    assert!(format!("{:?}", result).contains(
        "Err(Lib3hProtocolError(Other(\"No ghost message for request: \\\"client_to_lib3_response_"
    ),);
}

/// Test publish, Store, Query
#[allow(dead_code)]
fn test_author_and_hold(alex: &mut NodeMock, billy: &mut NodeMock, camille: &mut NodeMock) {
    // Hold an entry without publishing it
    println!("\nAlex broadcasts entry via GossipingList...\n");
    let entry_1 = alex
        .hold_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()])
        .unwrap();
    // Reply to the GetList request received from network module
    alex.reply_to_first_HandleGetGossipingEntryList();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    // Should receive a HandleFetchEntry request from network module after receiving list
    assert_eq!(srv_msg_list.len(), 1);
    // extract msg data
    let fetch_data = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::HandleFetchEntry);
    // Respond
    alex.reply_to_HandleFetchEntry(&fetch_data)
        .expect("Reply to HandleFetchEntry should work");
    let (did_work, _srv_msg_list) = alex.process().unwrap();
    assert!(did_work);

    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
    println!("\nBilly is told to hold it...\n");
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1, "{:?}", srv_msg_list);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::HandleStoreEntryAspect(response) = msg_1 {
        assert_eq!(response.entry_address, entry_1.entry_address);
    });
    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
    println!("\nCamille is told to hold it...\n");
    let (did_work, srv_msg_list) = camille.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1, "{:?}", srv_msg_list);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::HandleStoreEntryAspect(response) = msg_1 {
        assert_eq!(response.entry_address, entry_1.entry_address);
    });

    // Billy publish data on the network
    println!("\nBilly authors a second entry...\n");
    let entry_2 = billy
        .author_entry(&ENTRY_ADDRESS_2, vec![ASPECT_CONTENT_2.clone()], true)
        .unwrap();
    let (did_work, _srv_msg_list) = billy.process().unwrap();
    assert!(did_work);

    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
    println!("\nAlex is told to hold it...\n");
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1, "{:?}", srv_msg_list);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::HandleStoreEntryAspect(response) = msg_1 {
        assert_eq!(response.entry_address, entry_2.entry_address);
    });
    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
    println!("\nCamille is told to hold it...\n");
    let (did_work, srv_msg_list) = camille.process().unwrap();
    assert!(did_work);
    assert!(srv_msg_list.len() >= 1);

    // Camille requests 1st entry
    let enty_address_str = &entry_1.entry_address;
    println!(
        "\n{} requesting entry: {}\n",
        camille.name(),
        enty_address_str
    );
    let query_data = camille.request_entry(entry_1.entry_address.clone());
    let (did_work, _srv_msg_list) = camille.process().unwrap();
    assert!(did_work);
    // #fullsync
    // Billy sends that data back to the network
    println!("\n{} reply to own request:\n", camille.name());
    let _ = camille.reply_to_HandleQueryEntry(&query_data).unwrap();
    let (did_work, srv_msg_list) = camille.process().unwrap();
    println!(
        "\n{} gets own response {:?}\n",
        camille.name(),
        srv_msg_list
    );
    assert!(did_work);
    assert!(srv_msg_list.len() >= 1);
    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::QueryEntryResult);
    assert_eq!(&msg.entry_address, &entry_1.entry_address);
    let mut de = Deserializer::new(&msg.query_result[..]);
    let maybe_entry: Result<EntryData, rmp_serde::decode::Error> =
        Deserialize::deserialize(&mut de);
    let mut found_entry = maybe_entry.expect("Should have found an entry");
    found_entry.aspect_list.sort();
    assert_eq!(found_entry, entry_1);

    // Camille requests 2nd entry
    let enty_address_str = &entry_2.entry_address;
    println!(
        "\n{} requesting entry: {}\n",
        camille.name(),
        enty_address_str
    );
    let query_data = camille.request_entry(entry_2.entry_address.clone());
    let (did_work, _srv_msg_list) = camille.process().unwrap();
    assert!(did_work);
    // #fullsync
    // Billy sends that data back to the network
    println!("\n{} reply to own request:\n", camille.name());
    let _ = camille.reply_to_HandleQueryEntry(&query_data).unwrap();
    let (did_work, srv_msg_list) = camille.process().unwrap();
    println!(
        "\n{} gets own response {:?}\n",
        camille.name(),
        srv_msg_list
    );
    assert!(did_work);
    assert!(srv_msg_list.len() >= 1);
    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::QueryEntryResult);
    assert_eq!(&msg.entry_address, &entry_2.entry_address);
    let mut de = Deserializer::new(&msg.query_result[..]);
    let maybe_entry: Result<EntryData, rmp_serde::decode::Error> =
        Deserialize::deserialize(&mut de);
    let mut found_entry = maybe_entry.expect("Should have found an entry");
    found_entry.aspect_list.sort();
    assert_eq!(found_entry, entry_2);
}

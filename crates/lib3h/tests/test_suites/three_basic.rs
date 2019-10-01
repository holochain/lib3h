use crate::{node_mock::NodeMock, utils::constants::*};
use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol};
use rmp_serde::Deserializer;
use serde::Deserialize;

pub type ThreeNodesTestFn = fn(alex: &mut NodeMock, billy: &mut NodeMock, camille: &mut NodeMock);

lazy_static! {
    pub static ref THREE_NODES_BASIC_TEST_FNS: Vec<(ThreeNodesTestFn, bool)> = vec![
//        (test_setup_only, true),
//        (test_send_message, true),
        (test_author_and_hold, true),
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
    let (_did_work, _srv_msg_list) = camille.process().unwrap();

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
    let req_id = alex.send_direct_message(&BILLY_AGENT_ID, "wah".as_bytes().to_vec());
    let expected = "HandleSendDirectMessage\\(DirectMessageData \\{ space_address: HashString\\(\"\\w+\"\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"billy\"\\), from_agent_id: HashString\\(\"alex\"\\), content: \"wah\" \\}\\)";
    let results = assert2_msg_matches!(alex, billy, expected);
    let handle_send_direct_msg = results.first().unwrap();
    let event = handle_send_direct_msg.events.first().unwrap();
    let msg = unwrap_to!(event => Lib3hServerProtocol::HandleSendDirectMessage);
    let content = std::str::from_utf8(msg.content.as_slice()).unwrap();
    println!("HandleSendDirectMessage: {}", content);

    // C should not receive
    let expected = "None";
    let results = assert_msg_matches!(camille, expected);

    // Send response
    println!("\n Billy responds to Alex...\n");
    let response_content = format!("echo: {}", "wah").as_bytes().to_vec();
    trace!(
        "billy send response with msg.request_id={:?}",
        msg.request_id
    );
    billy.send_response(&msg.request_id, &alex.agent_id(), response_content.clone());
    let expected = "SendDirectMessageResult\\(DirectMessageData \\{ space_address: HashString\\(\"\\w+\"\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"alex\"\\), from_agent_id: HashString\\(\"billy\"\\), content: \"echo: wah\" \\}\\)";
    assert2_msg_matches!(alex, billy, expected);

    // C sends DM to A
    // ===============
    println!("\nCamille sends DM to Alex...\n");

    let req_id = camille.send_direct_message(&ALEX_AGENT_ID, "marco".as_bytes().to_vec());
    let expected = "HandleSendDirectMessage\\(DirectMessageData \\{ space_address: HashString\\(\"\\w+\"\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"alex\"\\), from_agent_id: HashString\\(\"camille\"\\), content: \"marco\" \\}\\)";
    let results = assert2_msg_matches!(alex, camille, expected);
    let handle_send_direct_msg = results.first().unwrap();
    let event = handle_send_direct_msg.events.first().unwrap();
    let msg = unwrap_to!(event => Lib3hServerProtocol::HandleSendDirectMessage);
    let content = std::str::from_utf8(msg.content.as_slice()).unwrap();
    println!("HandleSendDirectMessage: {}", content);

    // C should not receive
    let expected = "None";
    let results = assert_msg_matches!(billy, expected);

    // Send response
    println!("\n Alex responds to Camille...\n");
    let response_content = format!("echo: {}", content).as_bytes().to_vec();
    trace!(
        "billy send response with msg.request_id={:?}",
        msg.request_id
    );
    alex.send_response(&msg.request_id, &camille.agent_id(), response_content.clone());
    let expected = "SendDirectMessageResult\\(DirectMessageData \\{ space_address: HashString\\(\"\\w+\"\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"camille\"\\), from_agent_id: HashString\\(\"alex\"\\), content: \"echo: marco\" \\}\\)";
    assert2_msg_matches!(alex, camille, expected);
}

/// Test publish, Store, Query
fn test_author_and_hold(alex: &mut NodeMock, billy: &mut NodeMock, camille: &mut NodeMock) {
    // Hold an entry without publishing it
    println!("\n Alex broadcasts entry via GossipingList...\n");
    let entry_1 = alex
        .hold_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()])
        .unwrap();
    // Reply to the GetList request received from network module
    alex.reply_to_first_HandleGetGossipingEntryList();

    // Should receive a HandleFetchEntry request from network module after receiving list
    let expected = "HandleFetchEntry\\(FetchEntryData \\{ space_address: HashString\\(\"appA\"\\), entry_address: HashString\\(\"entry_addr_1\"\\), request_id: \"[\\w\\d_~]+\", provider_agent_id: HashString\\(\"alex\"\\), aspect_address_list: None \\}\\)";
    let results = assert2_msg_matches!(alex, billy, expected);
    let fetch_event = &results[0].events[0];
    // extract msg data
    let fetch_data = unwrap_to!(fetch_event => Lib3hServerProtocol::HandleFetchEntry);
    println!("fetch_data: {:?}", fetch_data);

    // #fullsync - mirrorDht will ask for content right away
    // Respond to fetch
    println!("Respond to fetch... ");
    alex.reply_to_HandleFetchEntry(&fetch_data)
        .expect("Reply to HandleFetchEntry should work");

    // Expect HandleStoreEntryAspect from receiving entry via gossip
    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: HashString\\(\"\\w+\"\\), provider_agent_id: HashString\\(\"billy\"\\), entry_address: HashString\\(\"entry_addr_1\"\\), entry_aspect: EntryAspectData \\{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"hello-1\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, billy, expected);
//    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: HashString\\(\"\\w+\"\\), provider_agent_id: HashString\\(\"billy\"\\), entry_address: HashString\\(\"entry_addr_1\"\\), entry_aspect: EntryAspectData \\{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"hello-1\", publish_ts: \\d+ \\} \\}\\)";
//    let _results = assert2_msg_matches!(alex, camille, expected);

//    // Should receive a HandleFetchEntry request from network module after receiving list
//    assert_eq!(srv_msg_list.len(), 1);
//    // extract msg data
//    let fetch_data = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::HandleFetchEntry);
//    // Respond
//    alex.reply_to_HandleFetchEntry(&fetch_data)
//        .expect("Reply to HandleFetchEntry should work");
//    let (did_work, _srv_msg_list) = alex.process().unwrap();
//    assert!(did_work);
//
//    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
//    println!("\n Billy is told to hold it...\n");
//    let (did_work, srv_msg_list) = billy.process().unwrap();
//    assert!(did_work);
//    assert_eq!(srv_msg_list.len(), 1, "{:?}", srv_msg_list);
//    let msg_1 = &srv_msg_list[0];
//    one_let!(Lib3hServerProtocol::HandleStoreEntryAspect(response) = msg_1 {
//        assert_eq!(response.entry_address, entry_1.entry_address);
//    });
//    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
//    println!("\n Camille is told to hold it...\n");
//    let (did_work, srv_msg_list) = camille.process().unwrap();
//    assert!(did_work);
//    assert_eq!(srv_msg_list.len(), 1, "{:?}", srv_msg_list);
//    let msg_1 = &srv_msg_list[0];
//    one_let!(Lib3hServerProtocol::HandleStoreEntryAspect(response) = msg_1 {
//        assert_eq!(response.entry_address, entry_1.entry_address);
//    });

//    // Billy publish data on the network
//    println!("\n Billy authors a second entry...\n");
//    let entry_2 = billy
//        .author_entry(&ENTRY_ADDRESS_2, vec![ASPECT_CONTENT_2.clone()], true)
//        .unwrap();
//    let (did_work, _srv_msg_list) = billy.process().unwrap();
//    assert!(did_work);
//
//    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
//    println!("\nAlex is told to hold it...\n");
//    let (did_work, srv_msg_list) = alex.process().unwrap();
//    assert!(did_work);
//    assert_eq!(srv_msg_list.len(), 1, "{:?}", srv_msg_list);
//    let msg_1 = &srv_msg_list[0];
//    one_let!(Lib3hServerProtocol::HandleStoreEntryAspect(response) = msg_1 {
//        assert_eq!(response.entry_address, entry_2.entry_address);
//    });
//    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
//    println!("\nCamille is told to hold it...\n");
//    let (did_work, srv_msg_list) = camille.process().unwrap();
//    assert!(did_work);
//    assert!(srv_msg_list.len() >= 1);
//
//    // Camille requests 1st entry
//    let enty_address_str = &entry_1.entry_address;
//    println!(
//        "\n{} requesting entry: {}\n",
//        camille.name(),
//        enty_address_str
//    );
//    let query_data = camille.request_entry(entry_1.entry_address.clone());
//    let (did_work, _srv_msg_list) = camille.process().unwrap();
//    assert!(did_work);
//    // #fullsync
//    // Billy sends that data back to the network
//    println!("\n{} reply to own request:\n", camille.name());
//    let _ = camille.reply_to_HandleQueryEntry(&query_data).unwrap();
//    let (did_work, srv_msg_list) = camille.process().unwrap();
//    println!(
//        "\n{} gets own response {:?}\n",
//        camille.name(),
//        srv_msg_list
//    );
//    assert!(did_work);
//    assert!(srv_msg_list.len() >= 1);
//    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::QueryEntryResult);
//    assert_eq!(&msg.entry_address, &entry_1.entry_address);
//    let mut de = Deserializer::new(&msg.query_result[..]);
//    let maybe_entry: Result<EntryData, rmp_serde::decode::Error> =
//        Deserialize::deserialize(&mut de);
//    let mut found_entry = maybe_entry.expect("Should have found an entry");
//    found_entry.aspect_list.sort();
//    assert_eq!(found_entry, entry_1);
//
//    // Camille requests 2nd entry
//    let enty_address_str = &entry_2.entry_address;
//    println!(
//        "\n{} requesting entry: {}\n",
//        camille.name(),
//        enty_address_str
//    );
//    let query_data = camille.request_entry(entry_2.entry_address.clone());
//    let (did_work, _srv_msg_list) = camille.process().unwrap();
//    assert!(did_work);
//    // #fullsync
//    // Billy sends that data back to the network
//    println!("\n{} reply to own request:\n", camille.name());
//    let _ = camille.reply_to_HandleQueryEntry(&query_data).unwrap();
//    let (did_work, srv_msg_list) = camille.process().unwrap();
//    println!(
//        "\n{} gets own response {:?}\n",
//        camille.name(),
//        srv_msg_list
//    );
//    assert!(did_work);
//    assert!(srv_msg_list.len() >= 1);
//    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::QueryEntryResult);
//    assert_eq!(&msg.entry_address, &entry_2.entry_address);
//    let mut de = Deserializer::new(&msg.query_result[..]);
//    let maybe_entry: Result<EntryData, rmp_serde::decode::Error> =
//        Deserialize::deserialize(&mut de);
//    let mut found_entry = maybe_entry.expect("Should have found an entry");
//    found_entry.aspect_list.sort();
//    assert_eq!(found_entry, entry_2);
}

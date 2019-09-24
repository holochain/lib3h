use crate::{node_mock::NodeMock, utils::constants::*};
use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol, Address};
use rmp_serde::Deserializer;
use serde::Deserialize;

pub type TwoNodesTestFn = fn(alex: &mut NodeMock, billy: &mut NodeMock);

lazy_static! {
    pub static ref TWO_NODES_BASIC_TEST_FNS: Vec<(TwoNodesTestFn, bool)> = vec![
        (test_setup_only, true),
        (test_send_message, true),
        (test_send_message_fail, true),
        (test_hold_entry, true),
        (test_author_no_aspect, true),
        (test_author_one_aspect, true),
        (test_author_two_aspects, true),
        (test_two_authors, true),
    ];
}

//--------------------------------------------------------------------------------------------------
// Test setup
//--------------------------------------------------------------------------------------------------

///
pub fn setup_two_nodes(mut alex: &mut NodeMock, mut billy: &mut NodeMock) {
    // Connect Alex to Billy
    let connect_data = alex.connect_to(&billy.advertise()).unwrap();
    wait_connect!(alex, connect_data, billy);

    billy.wait_until_no_work();
    alex.wait_until_no_work();
    billy.wait_until_no_work();
    two_join_space(&mut alex, &mut billy, &SPACE_ADDRESS_A);

    println!("DONE setup_two_nodes() DONE \n\n\n");
}

//--------------------------------------------------------------------------------------------------
// Helpers
//--------------------------------------------------------------------------------------------------

/// Request ENTRY_ADDRESS_1 from the network and should get it back
pub fn request_entry_ok(node: &mut NodeMock, entry: &EntryData) {
    let enty_address_str = &entry.entry_address;
    println!("\n{} requesting entry: {}\n", node.name(), enty_address_str);
    let query_data = node.request_entry(entry.entry_address.clone());
    let (did_work, srv_msg_list) = node.process().unwrap();
    assert!(did_work);
    println!("\n srv_msg_list: {:?}", srv_msg_list);

    // #fullsync
    // Billy sends that data back to the network
    println!("\n{} reply to own request:\n", node.name());
    let _ = node.reply_to_HandleQueryEntry(&query_data).unwrap();
    let (did_work, srv_msg_list) = node.process().unwrap();
    println!("\n{} gets own response {:?}\n", node.name(), srv_msg_list);
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1, "{:?}", srv_msg_list);
    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::QueryEntryResult);
    assert_eq!(&msg.entry_address, &entry.entry_address);
    let mut de = Deserializer::new(&msg.query_result[..]);
    let maybe_entry: Result<EntryData, rmp_serde::decode::Error> =
        Deserialize::deserialize(&mut de);
    let mut found_entry = maybe_entry.expect("Should have found an entry");
    found_entry.aspect_list.sort();
    assert_eq!(&found_entry, entry);
}

///
pub fn two_join_space(alex: &mut NodeMock, billy: &mut NodeMock, space_address: &Address) {
    println!(
        "\ntwo_join_space ({},{}) -> {}\n",
        alex.name(),
        billy.name(),
        space_address
    );
    // Alex joins space
    let req_id = alex.join_space(&space_address, true).unwrap();
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
    println!("\n {} joins {}\n", billy.name(), space_address);
    let req_id = billy.join_space(&space_address, true).unwrap();
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 3);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::SuccessResult(response) = msg_1 {
        assert_eq!(response.request_id, req_id);
    });
    // Extra processing required for auto-handshaking
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

/// Empty function that triggers the test suite
fn test_setup_only(_alex: &mut NodeMock, _billy: &mut NodeMock) {
    // n/a
}

/// Test SendDirectMessage and response
pub fn test_send_message(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Send DM
    //let req_id = alex.send_direct_message(&BILLY_AGENT_ID, "wah".as_bytes().to_vec());

    let expected = "HandleSendDirectMessage\\(DirectMessageData \\{ space_address: HashString\\(\"SPACE_A\"\\), request_id: \"client_to_lib3_response[\\w\\d_~]+\", to_agent_id: HashString\\(\"billy\"\\), from_agent_id: HashString\\(\"alex\"\\), content: \"wah\" \\}\\)";

    let results = assert2_msg_matches!(alex, billy, expected);

    let handle_send_direct_msg = results.first().unwrap();

    let event = handle_send_direct_msg.events.first().unwrap();

    let msg = unwrap_to!(event => Lib3hServerProtocol::HandleSendDirectMessage);

    // Send response
    let response_content = format!("echo: {}", "wah").as_bytes().to_vec();
    billy.send_response(&msg.request_id, &alex.agent_id(), response_content.clone());

    // TODO Set this to correct value once test passes
    let expected = "Lib3hServerProtocol::SendDirectMessageResult";

    assert2_msg_matches!(alex, billy, expected);
}

/// Test SendDirectMessage and response
fn test_send_message_fail(alex: &mut NodeMock, _billy: &mut NodeMock) {
    // Send to self
    let req_id = alex.send_direct_message(&ALEX_AGENT_ID, "wah".as_bytes().to_vec());
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    println!("response: {:?}", srv_msg_list);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::FailureResult(response) = msg_1 {
        assert_eq!(response.request_id, req_id);
    });

    // Send to unknown
    let req_id = alex.send_direct_message(&CAMILLE_AGENT_ID, "wah".as_bytes().to_vec());
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    println!("response: {:?}", srv_msg_list);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::FailureResult(response) = msg_1 {
        assert_eq!(response.request_id, req_id);
    });
}

/// Test publish, Store, Query
pub fn test_author_one_aspect(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Alex publish data on the network
    let entry = alex
        .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
        .unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);

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
    request_entry_ok(billy, &entry);

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
fn test_hold_entry(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Alex holds an entry
    let entry = alex
        .hold_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
        .unwrap();
    let (did_work, _srv_msg_list) = alex.process().unwrap();
    assert!(did_work);

    // Process the HoldEntry generated from receiving HandleStoreEntryAspect
    println!("\nBilly should receive entry from gossip and asks owner to validate it:\n");
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    println!("\n srv_msg_list: {:?}", srv_msg_list);
    println!("\nBilly should process the HoldEntry from NodeMock auto-validation:\n");
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    println!("\n srv_msg_list: {:?}", srv_msg_list);
    // Billy asks for that entry
    request_entry_ok(billy, &entry);

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

/// Entry with no Aspect case: Should no-op
fn test_author_no_aspect(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Alex publish data on the network
    alex.author_entry(&ENTRY_ADDRESS_1, vec![], true).unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 0);

    // #fullsync
    // Alex or Billy should not receive anything
    let store_result = billy.wait_with_timeout(
        Box::new(one_is!(Lib3hServerProtocol::HandleStoreEntryAspect(_))),
        1000,
    );
    assert!(store_result.is_none());
    let (_did_work, srv_msg_list) = billy.process().unwrap();
    assert_eq!(srv_msg_list.len(), 0);
}

/// Entry with two aspects case
fn test_author_two_aspects(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Alex authors and broadcast an entry on the space
    let entry = alex
        .author_entry(
            &ENTRY_ADDRESS_1,
            vec![ASPECT_CONTENT_1.clone(), ASPECT_CONTENT_2.clone()],
            true,
        )
        .unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 2);

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
    request_entry_ok(billy, &entry);
}

/// Entry with two aspects case
fn test_two_authors(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Alex authors and broadcast first aspect
    // =======================================
    let _ = alex
        .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
        .unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);

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

    // Billy authors and broadcast second aspect
    // =========================================
    let _ = billy
        .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_2.clone()], true)
        .unwrap();
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);

    // #fullsync
    // Alex or Billy should receive the entry store request
    let store_result = alex.wait(Box::new(one_is!(
        Lib3hServerProtocol::HandleStoreEntryAspect(_)
    )));
    assert!(store_result.is_some());
    println!("\n got HandleStoreEntryAspect: {:?}", store_result);
    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
    let (did_work, _srv_msg_list) = alex.process().unwrap();
    assert!(did_work);

    // Alex asks for that entry
    let entry = NodeMock::form_EntryData(
        &ENTRY_ADDRESS_1,
        vec![ASPECT_CONTENT_1.clone(), ASPECT_CONTENT_2.clone()],
    );
    request_entry_ok(alex, &entry);
}

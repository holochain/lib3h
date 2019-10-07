use crate::{
    node_mock::{test_join_space, NodeMock},
    utils::{constants::*, processor_harness::ProcessingOptions},
};
use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol, types::*};
use rmp_serde::Deserializer;
use serde::Deserialize;

pub type TwoNodesTestFn =
    fn(alex: &mut NodeMock, billy: &mut NodeMock, options: &ProcessingOptions);

lazy_static! {
    pub static ref TWO_NODES_BASIC_TEST_FNS: Vec<(TwoNodesTestFn, bool)> = vec![
        (test_setup_only, true),
        (test_send_message, true),
        //(test_send_message_fail, true),
        (test_send_message_self, true),
        (test_author_no_aspect, true),
        (test_author_one_aspect, true),
        (test_author_two_aspects, true),
        (test_two_authors, true),
    ];
}

//--------------------------------------------------------------------------------------------------
// Test setup
//--------------------------------------------------------------------------------------------------

#[allow(dead_code)]
pub fn setup_two_nodes(mut alex: &mut NodeMock, mut billy: &mut NodeMock, options: &ProcessingOptions) {
    // Connect Alex to Billy
    let connect_data = alex.connect_to(&billy.advertise()).unwrap();
    wait_connect!(alex, connect_data, billy);

    billy.wait_until_no_work();
    alex.wait_until_no_work();
    billy.wait_until_no_work();
    two_join_space(&mut alex, &mut billy, &SPACE_ADDRESS_A, options);

    debug!(
        "DONE setup_two_nodes() DONE \n\n ------------------------------------------------ \n"
    );
}

//--------------------------------------------------------------------------------------------------
// Helpers
//--------------------------------------------------------------------------------------------------

/// Request ENTRY_ADDRESS_1 from the network and should get it back
#[allow(dead_code)]
pub fn request_entry_ok(node: &mut NodeMock, entry: &EntryData, options: &ProcessingOptions) {
    let enty_address_str = &entry.entry_address;
    debug!("\n{} requesting entry: {}\n", node.name(), enty_address_str);
    let mut query_data = node.request_entry(entry.entry_address.clone());

    let expected = "HandleQueryEntry\\(QueryEntryData \\{ space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), entry_address: HashString\\(\"[\\w\\d_~]+\"\\), request_id: \"[\\w\\d_~]+\", requester_agent_id: HashString\\(\"[\\w\\d]+\"\\), query: \"test_query\" \\}\\)";
    let results = assert_msg_matches!(node, expected, options);
    debug!("\n results: {:?}\n", results);
    let handle_query = &results[0].events[0];
    debug!("\n query_data: {:?}\n", query_data);
    debug!("\n handle_query_data: {:?}\n", handle_query);
    if let Lib3hServerProtocol::HandleQueryEntry(h_query_data) = handle_query {
        query_data = h_query_data.to_owned();
    }

    // #fullsync
    // Billy sends that data back to the network
    debug!("\n{} reply to own request: {:?}\n", node.name(), query_data);
    let _ = node.reply_to_HandleQueryEntry(&query_data).unwrap();

    let expected = "QueryEntryResult\\(QueryEntryResultData \\{ space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), entry_address: HashString\\(\"[\\w\\d_~]+\"\\), request_id: \"[\\w\\d_~]+\", requester_agent_id: HashString\\(\"[\\w\\d]+\"\\), responder_agent_id: HashString\\(\"[\\w\\d]+\"\\), query_result: ";

    let results = assert_msg_matches!(node, expected, options);
    debug!("\n results: {:?}\n", results);
    let query_result = &results[0].events[0];
    debug!("\n query_result: {:?}\n", query_result);
    let msg = unwrap_to!(query_result => Lib3hServerProtocol::QueryEntryResult);
    assert_eq!(&msg.entry_address, &entry.entry_address);

    let mut de = Deserializer::new(&msg.query_result[..]);
    let maybe_entry: Result<EntryData, rmp_serde::decode::Error> =
        Deserialize::deserialize(&mut de);
    let mut found_entry = maybe_entry.expect("Should have found an entry");
    found_entry.aspect_list.sort();
    assert_eq!(&found_entry, entry);
}

// setup for two nodes joining the same space
pub fn two_join_space(alex: &mut NodeMock, billy: &mut NodeMock, space_address: &SpaceHash, options: &ProcessingOptions) {
    debug!(
        "\ntwo_join_space ({},{}) -> {}\n",
        alex.name(),
        billy.name(),
        space_address
    );
    // Alex joins space
    test_join_space(alex, space_address, options);
    // Billy joins space
    test_join_space(billy, space_address, options);

    // Extra processing required for auto-handshaking
    // TODO figure out something to explicitly wait on (eg. a drained message)
    wait_engine_wrapper_until_no_work!(alex);
    wait_engine_wrapper_until_no_work!(billy);
    wait_engine_wrapper_until_no_work!(alex);
    wait_engine_wrapper_until_no_work!(billy);
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

/// Empty function that triggers the test suite
#[allow(dead_code)]
fn test_setup_only(_alex: &mut NodeMock, _billy: &mut NodeMock, _options: &ProcessingOptions) {
    // n/a
}

/// Test SendDirectMessage and response
pub fn test_send_message(alex: &mut NodeMock, billy: &mut NodeMock, options: &ProcessingOptions) {
    // Send DM
    let _req_id = alex.send_direct_message(&BILLY_AGENT_ID, "wah".as_bytes().to_vec());

    let expected = "HandleSendDirectMessage\\(DirectMessageData \\{ space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"billy\"\\), from_agent_id: HashString\\(\"alex\"\\), content: \"wah\" \\}\\)";
    let results = assert2_msg_matches!(alex, billy, expected, options);
    let handle_send_direct_msg = results.first().unwrap();
    let event = handle_send_direct_msg.events.first().unwrap();
    let msg = unwrap_to!(event => Lib3hServerProtocol::HandleSendDirectMessage);

    // Send response
    let response_content = format!("echo: {}", "wah").as_bytes().to_vec();
    trace!(
        "billy send response with msg.request_id={:?}",
        msg.request_id
    );
    billy.send_response(&msg.request_id, &alex.agent_id(), response_content.clone());

    let expected = "SendDirectMessageResult\\(DirectMessageData \\{ space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"alex\"\\), from_agent_id: HashString\\(\"billy\"\\), content: \"echo: wah\" \\}\\)";
    assert2_msg_matches!(alex, billy, expected, options);
}

/// Test SendDirectMessage and response failure
#[allow(dead_code)]
fn test_send_message_fail(alex: &mut NodeMock, _billy: &mut NodeMock, options: &ProcessingOptions) {
    trace!("[test_send_message_fail] alex send to camille");
    // Send to unknown
    let _req_id = alex.send_direct_message(&CAMILLE_AGENT_ID, "wah".as_bytes().to_vec());

    let expected = "FailureResult\\(GenericResultData \\{ request_id: \"req_alex_3\", space_address: SpaceHash\\(HashString\\(\"appA\"\\), to_agent_id: HashString\\(\"camille\"\\), result_info: ";
    assert_msg_matches!(alex, expected, options);
}

/// Test SendDirectMessage and response to self
pub fn test_send_message_self(
    alex: &mut NodeMock,
    _billy: &mut NodeMock,
    options: &ProcessingOptions,
) {
    // Send DM
    let _req_id = alex.send_direct_message(&ALEX_AGENT_ID, "wah".as_bytes().to_vec());

    let expected = "HandleSendDirectMessage\\(DirectMessageData \\{ space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"alex\"\\), from_agent_id: HashString\\(\"alex\"\\), content: \"wah\" \\}\\)";

    let results = assert_msg_matches!(alex, expected, options);

    let handle_send_direct_msg = results.first().unwrap();

    let event = handle_send_direct_msg.events.first().unwrap();

    let msg = unwrap_to!(event => Lib3hServerProtocol::HandleSendDirectMessage);

    // Send response
    let response_content = format!("echo: {}", "wah").as_bytes().to_vec();
    trace!(
        "alex send response with msg.request_id={:?}",
        msg.request_id
    );
    alex.send_response(&msg.request_id, &alex.agent_id(), response_content.clone());

    // TODO Set this to correct value once test passes
    let expected = "SendDirectMessageResult\\(DirectMessageData \\{ space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"alex\"\\), from_agent_id: HashString\\(\"alex\"\\), content: \"echo: wah\" \\}\\)";

    assert_msg_matches!(alex, expected, options);
}

/// Test publish, Store, Query
#[allow(dead_code)]
pub fn test_author_one_aspect(
    alex: &mut NodeMock,
    billy: &mut NodeMock,
    options: &ProcessingOptions,
) {
    // Alex publish data on the network
    let entry = alex
        .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
        .unwrap();

    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), provider_agent_id: HashString\\(\"billy\"\\), entry_address: HashString\\(\"entry_addr_1\"\\), entry_aspect: EntryAspectData \\{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"hello-1\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, billy, expected, options);

    // Billy asks for that entry
    // =========================
    request_entry_ok(billy, &entry, options);

    // Billy asks for unknown entry
    // ============================
    let mut query_data = billy.request_entry(ENTRY_ADDRESS_2.clone());
    let expected = "HandleQueryEntry\\(QueryEntryData \\{ space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), entry_address: HashString\\(\"entry_addr_2\"\\), request_id: \"[\\w\\d_~]+\", requester_agent_id: HashString\\(\"billy\"\\), query: \"test_query\" \\}\\)";
    let results = assert2_msg_matches!(alex, billy, expected, options);
    println!("\n results: {:?}\n", results);
    let handle_query = &results[0].events[0];
    println!("\n query_data: {:?}\n", query_data);
    println!("\n handle_query_data: {:?}\n", handle_query);
    if let Lib3hServerProtocol::HandleQueryEntry(h_query_data) = handle_query {
        query_data = h_query_data.to_owned();
    }

    // Expecting an empty entry
    let res = billy.reply_to_HandleQueryEntry(&query_data);
    println!("\n billy gives response {:?}\n", res);
    assert!(res.is_ok());
    let result_data = res.unwrap();
    assert_eq!(result_data.entry_address, *ENTRY_ADDRESS_2);
    let opaque_result: Vec<u8> = result_data.query_result.into();
    let expected: Vec<u8> = [
        146, 145, 172, 101, 110, 116, 114, 121, 95, 97, 100, 100, 114, 95, 50, 144,
    ]
    .to_vec();
    assert_eq!(opaque_result, expected);
}

/// Entry with no Aspect case: Should no-op
#[allow(dead_code)]
fn test_author_no_aspect(alex: &mut NodeMock, billy: &mut NodeMock, _options: &ProcessingOptions) {
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
#[allow(dead_code)]
fn test_author_two_aspects(alex: &mut NodeMock, billy: &mut NodeMock, options: &ProcessingOptions) {
    // Alex authors and broadcast an entry on the space
    let _entry = alex
        .author_entry(
            &ENTRY_ADDRESS_1,
            vec![ASPECT_CONTENT_1.clone(), ASPECT_CONTENT_2.clone()],
            true,
        )
        .unwrap();
    let (_did_work, srv_msg_list) = alex.process().unwrap();
    assert_eq!(srv_msg_list.len(), 2);

    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), provider_agent_id: HashString\\(\"billy\"\\), entry_address: HashString\\(\"entry_addr_1\"\\), entry_aspect: EntryAspectData \\{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"[\\w\\d\\-]+\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, billy, expected, options);
    let mut entry = billy.get_entry(&ENTRY_ADDRESS_1).unwrap();
    entry.aspect_list.sort();
    assert_eq!(entry.aspect_list.len(), 2);

    // Billy asks for that entry
    request_entry_ok(billy, &entry, options);
}

/// Entry with two aspects case
#[allow(dead_code)]
fn test_two_authors(alex: &mut NodeMock, billy: &mut NodeMock, options: &ProcessingOptions) {
    // Alex authors and broadcast first aspect
    // =======================================
    let _ = alex
        .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
        .unwrap();
    let (_did_work, srv_msg_list) = alex.process().unwrap();
    assert_eq!(srv_msg_list.len(), 1);

    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), provider_agent_id: HashString\\(\"billy\"\\), entry_address: HashString\\(\"entry_addr_1\"\\), entry_aspect: EntryAspectData \\{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"[\\w\\d\\-]+\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, billy, expected, options);

    // Billy authors and broadcast second aspect
    // =========================================
    let _ = billy
        .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_2.clone()], true)
        .unwrap();
    let (_did_work, srv_msg_list) = billy.process().unwrap();
    assert_eq!(srv_msg_list.len(), 1);

    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), provider_agent_id: HashString\\(\"[\\w\\d]+\"\\), entry_address: HashString\\(\"entry_addr_1\"\\), entry_aspect: EntryAspectData \\{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"[\\w\\d\\-]+\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, billy, expected, options);

    // Alex asks for that entry
    let entry = NodeMock::form_EntryData(
        &ENTRY_ADDRESS_1,
        vec![ASPECT_CONTENT_1.clone(), ASPECT_CONTENT_2.clone()],
    );
    request_entry_ok(alex, &entry, options);
}

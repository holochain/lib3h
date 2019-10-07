use crate::{
    node_mock::{test_join_space, NodeMock},
    test_suites::two_basic::request_entry_ok,
    utils::{processor_harness::ProcessingOptions, constants::*},
};
use lib3h_protocol::protocol_server::Lib3hServerProtocol;

pub type ThreeNodesTestFn = fn(alex: &mut NodeMock, billy: &mut NodeMock, camille: &mut NodeMock,
                               options: &ProcessingOptions);

lazy_static! {
    pub static ref THREE_NODES_BASIC_TEST_FNS: Vec<(ThreeNodesTestFn, bool)> = vec![
        (test_setup_only, true),
        (test_send_message, true),
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
    options: &ProcessingOptions
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

    // Space joining
    // =============
    // Alex joins space
    test_join_space(alex, &SPACE_ADDRESS_A, options);

    // Billy joins space
    test_join_space(billy, &SPACE_ADDRESS_A, options);

    // Camille joins space
    test_join_space(camille, &SPACE_ADDRESS_A, options);

    // Extra processing required for auto-handshaking
    wait_engine_wrapper_until_no_work!(alex);
    wait_engine_wrapper_until_no_work!(billy);
    wait_engine_wrapper_until_no_work!(camille);
    wait_engine_wrapper_until_no_work!(alex);
    wait_engine_wrapper_until_no_work!(billy);
    wait_engine_wrapper_until_no_work!(camille);

    debug!("DONE setup_three_nodes() DONE \n\n ============================================ \n");
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

/// Empty function that triggers the test suite
fn test_setup_only(_alex: &mut NodeMock, _billy: &mut NodeMock, _camille: &mut NodeMock, _options: &ProcessingOptions) {
    // n/a
}

/// Test SendDirectMessage and response
fn test_send_message(alex: &mut NodeMock, billy: &mut NodeMock, camille: &mut NodeMock, options: &ProcessingOptions) {
    // A sends DM to B
    // ===============
    let _req_id = alex.send_direct_message(&BILLY_AGENT_ID, "wah".as_bytes().to_vec());
    let expected = "HandleSendDirectMessage\\(DirectMessageData \\{ space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"billy\"\\), from_agent_id: HashString\\(\"alex\"\\), content: \"wah\" \\}\\)";
    let results = assert2_msg_matches!(alex, billy, expected, options);
    let handle_send_direct_msg = results.first().unwrap();
    let event = handle_send_direct_msg.events.first().unwrap();
    let msg = unwrap_to!(event => Lib3hServerProtocol::HandleSendDirectMessage);
    let content = std::str::from_utf8(msg.content.as_slice()).unwrap();
    debug!("HandleSendDirectMessage: {}", content);

    // C should not receive
    let expected = "None";
    let _results = assert_msg_matches!(camille, expected, options);

    // Send response
    debug!("\n Billy responds to Alex...\n");
    let response_content = format!("echo: {}", "wah").as_bytes().to_vec();
    trace!(
        "billy send response with msg.request_id={:?}",
        msg.request_id
    );
    billy.send_response(&msg.request_id, &alex.agent_id(), response_content.clone());
    let expected = "SendDirectMessageResult\\(DirectMessageData \\{ space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"alex\"\\), from_agent_id: HashString\\(\"billy\"\\), content: \"echo: wah\" \\}\\)";
    assert2_msg_matches!(alex, billy, expected, options);

    // C sends DM to A
    // ===============
    debug!("\nCamille sends DM to Alex...\n");

    let _req_id = camille.send_direct_message(&ALEX_AGENT_ID, "marco".as_bytes().to_vec());
    let expected = "HandleSendDirectMessage\\(DirectMessageData \\{ space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"alex\"\\), from_agent_id: HashString\\(\"camille\"\\), content: \"marco\" \\}\\)";
    let results = assert2_msg_matches!(alex, camille, expected, options);
    let handle_send_direct_msg = results.first().unwrap();
    let event = handle_send_direct_msg.events.first().unwrap();
    let msg = unwrap_to!(event => Lib3hServerProtocol::HandleSendDirectMessage);
    let content = std::str::from_utf8(msg.content.as_slice()).unwrap();
    println!("HandleSendDirectMessage: {}", content);

    // C should not receive
    let expected = "None";
    let _results = assert_msg_matches!(billy, expected, options);

    // Send response
    println!("\n Alex responds to Camille...\n");
    let response_content = format!("echo: {}", content).as_bytes().to_vec();
    trace!(
        "alex send response with msg.request_id={:?}",
        msg.request_id
    );

    alex.send_response(
        &msg.request_id,
        &camille.agent_id(),
        response_content.clone(),
    );
    let expected = "SendDirectMessageResult\\(DirectMessageData \\{ space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), request_id: \"[\\w\\d_~]+\", to_agent_id: HashString\\(\"camille\"\\), from_agent_id: HashString\\(\"alex\"\\), content: \"echo: marco\" \\}\\)";
    assert2_msg_matches!(alex, camille, expected);
}

/// Test publish, Store, Query
#[allow(dead_code)]
fn test_author_and_hold(alex: &mut NodeMock, billy: &mut NodeMock, camille: &mut NodeMock,
                        options: &ProcessingOptions) {
    // Hold an entry without publishing it
    println!("\n Alex broadcasts entry via GossipingList...\n");
    let entry_1 = alex
        .hold_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()])
        .unwrap();
    // Reply to the GetList request received from network module
    alex.reply_to_first_HandleGetGossipingEntryList();

    // Should receive a HandleFetchEntry request from network module after receiving list
    let expected = "HandleFetchEntry\\(FetchEntryData \\{ space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), entry_address: HashString\\(\"entry_addr_1\"\\), request_id: \"[\\w\\d_~]+\", provider_agent_id: HashString\\(\"alex\"\\), aspect_address_list: None \\}\\)";
    let results = assert_msg_matches!(alex, expected, options);
    let fetch_event = &results[0].events[0];
    // extract msg data
    let fetch_data = unwrap_to!(fetch_event => Lib3hServerProtocol::HandleFetchEntry);
    debug!("fetch_data: {:?}", fetch_data);

    // #fullsync - mirrorDht will ask for content right away
    // Respond to fetch
    debug!("Respond to fetch... ");
    alex.reply_to_HandleFetchEntry(&fetch_data)
        .expect("Reply to HandleFetchEntry should work");

    // Expect HandleStoreEntryAspect from receiving entry via gossip
    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), provider_agent_id: HashString\\(\"billy\"\\), entry_address: HashString\\(\"entry_addr_1\"\\), entry_aspect: EntryAspectData \\{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"hello-1\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, billy, expected, options);
    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), provider_agent_id: HashString\\(\"camille\"\\), entry_address: HashString\\(\"entry_addr_1\"\\), entry_aspect: EntryAspectData \\{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"hello-1\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, camille, expected, options);

    // Billy publish data on the network
    println!("\n Billy authors a second entry...\n");
    let entry_2 = billy
        .author_entry(&ENTRY_ADDRESS_2, vec![ASPECT_CONTENT_2.clone()], true)
        .unwrap();
    // let (did_work, _srv_msg_list) = billy.process().unwrap();

    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), provider_agent_id: HashString\\(\"alex\"\\), entry_address: HashString\\(\"entry_addr_2\"\\), entry_aspect: EntryAspectData \\{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"l-2\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, billy, expected, options);

    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), provider_agent_id: HashString\\(\"camille\"\\), entry_address: HashString\\(\"entry_addr_2\"\\), entry_aspect: EntryAspectData \\{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"l-2\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(camille, billy, expected, options);

    request_entry_ok(camille, &entry_1, options);
    request_entry_ok(camille, &entry_2, options);
}

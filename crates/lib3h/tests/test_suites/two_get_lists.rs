use crate::{
    node_mock::NodeMock,
    test_suites::two_basic::{request_entry_ok, TwoNodesTestFn},
    utils::{constants::*, processor_harness::ProcessingOptions},
};
use lib3h_protocol::protocol_server::Lib3hServerProtocol;

lazy_static! {
    pub static ref TWO_NODES_GET_LISTS_TEST_FNS: Vec<(TwoNodesTestFn, bool)> = vec![
        (author_list_test, true),
        (hold_list_test, true),
        (empty_author_list_test, true),
        (author_list_known_entry_test, true),
        (many_aspects_test, true),
    ];
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

/// Return some entry in authoring_list request
pub fn author_list_test(alex: &mut NodeMock, billy: &mut NodeMock, options: &ProcessingOptions) {
    // author an entry without publishing it
    let entry = alex
        .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], false)
        .unwrap();
    // Reply to the publish_list request received from network module
    alex.reply_to_first_HandleGetAuthoringEntryList();

    // Should receive a HandleFetchEntry request from network module after receiving list
    let expected = "HandleFetchEntry\\(FetchEntryData \\{ space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), entry_address: EntryHash\\(HashString\\(\"entry_addr_1\"\\)\\), request_id: \"[\\w\\d_~]+\", provider_agent_id: AgentPubKey\\(HashString\\(\"alex\"\\)\\), aspect_address_list: None \\}\\)";
    let results = assert2_msg_matches!(alex, billy, expected, options);
    let fetch_event = &results[0].events[0];
    // extract msg data
    let fetch_data = unwrap_to!(fetch_event => Lib3hServerProtocol::HandleFetchEntry);
    println!("fetch_data: {:?}", fetch_data);
    // Respond
    alex.reply_to_HandleFetchEntry(&fetch_data)
        .expect("Reply to HandleFetchEntry should work");

    // Expecting a HandleStoreEntryAspect
    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), provider_agent_id: AgentPubKey\\(HashString\\(\"billy\"\\)\\), entry_address: EntryHash\\(HashString\\(\"entry_addr_1\"\\)\\), entry_aspect: EntryAspectData \\{ aspect_address: AspectHash\\(HashString\\(\"[\\w\\d]+\"\\)\\), type_hint: \"NodeMock\", aspect: \"hello-1\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, billy, expected, options);

    // Billy asks for that entry
    request_entry_ok(billy, &entry, options);
}

/// Return some entry in gossiping_list request
pub fn hold_list_test(alex: &mut NodeMock, billy: &mut NodeMock, options: &ProcessingOptions) {
    // Have alex hold some data
    let entry = alex
        .hold_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()])
        .unwrap();
    // Alex: Look for the hold_list request received from network module and reply
    alex.reply_to_first_HandleGetGossipingEntryList();

    // Should receive a HandleFetchEntry request from network module after receiving list
    let expected = "HandleFetchEntry\\(FetchEntryData \\{ space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), entry_address: EntryHash\\(HashString\\(\"entry_addr_1\"\\)\\), request_id: \"[\\w\\d_~]+\", provider_agent_id: AgentPubKey\\(HashString\\(\"alex\"\\)\\), aspect_address_list: None \\}\\)";
    let results = assert2_msg_matches!(alex, billy, expected, options);
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
    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), provider_agent_id: AgentPubKey\\(HashString\\(\"billy\"\\)\\), entry_address: EntryHash\\(HashString\\(\"entry_addr_1\"\\)\\), entry_aspect: EntryAspectData \\{ aspect_address: AspectHash\\(HashString\\(\"[\\w\\d]+\"\\)\\), type_hint: \"NodeMock\", aspect: \"hello-1\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, billy, expected, options);

    // Billy asks for that entry
    request_entry_ok(billy, &entry, options);
}

///
pub fn empty_author_list_test(
    alex: &mut NodeMock,
    billy: &mut NodeMock,
    options: &ProcessingOptions,
) {
    // Alex replies an empty list to the initial HandleGetAuthoringEntryList
    alex.reply_to_first_HandleGetAuthoringEntryList();

    let expected = "None";
    let _results = assert2_msg_matches!(alex, billy, expected, options);

    // Billy asks for unpublished data.
    println!("\n{} requesting entry: ENTRY_ADDRESS_1\n", billy.name());
    let _query_data = billy.request_entry(ENTRY_ADDRESS_1.clone());

    // Receives back the HandleQuery
    let expected = "HandleQueryEntry\\(QueryEntryData \\{ space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), entry_address: EntryHash\\(HashString\\(\"entry_addr_1\"\\)\\), request_id: \"[\\w\\d_~]+\", requester_agent_id: AgentPubKey\\(HashString\\(\"billy\"\\)\\), query: \"test_query\" \\}\\)";
    let results = assert2_msg_matches!(alex, billy, expected, options);
    let query_event = &results[0].events[0];
    // extract msg data
    let query_data = unwrap_to!(query_event => Lib3hServerProtocol::HandleQueryEntry);
    println!("query_data: {:?}", query_data);
    // #fullsync
    // Alex sends back an empty response to the network
    println!("\n{} looking for HandleQueryEntry\n", billy.name());
    let res = billy.reply_to_HandleQueryEntry(query_data);
    println!("\n{} found: {:?}\n", billy.name(), res);
    assert!(res.is_ok());
    let result_data = res.unwrap();
    assert_eq!(result_data.entry_address, *ENTRY_ADDRESS_1);
    let opaque_result: Vec<u8> = result_data.query_result.into();
    let expected: Vec<u8> = [
        146, 145, 145, 172, 101, 110, 116, 114, 121, 95, 97, 100, 100, 114, 95, 49, 144,
    ]
    .to_vec();
    assert_eq!(opaque_result, expected);
}

/// Return author_list with already known entry
pub fn author_list_known_entry_test(
    alex: &mut NodeMock,
    billy: &mut NodeMock,
    options: &ProcessingOptions,
) {
    let entry = alex
        .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
        .unwrap();
    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), provider_agent_id: AgentPubKey\\(HashString\\(\"billy\"\\)\\), entry_address: EntryHash\\(HashString\\(\"entry_addr_1\"\\)\\), entry_aspect: EntryAspectData \\{ aspect_address: AspectHash\\(HashString\\(\"[\\w\\d]+\"\\)\\), type_hint: \"NodeMock\", aspect: \"hello-1\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, billy, expected, options);

    alex.reply_to_first_HandleGetAuthoringEntryList();
    // Should not receive a HandleFetchEntry request from network module after receiving list
    let expected = "None";
    let fast_timeout_options = ProcessingOptions {
        max_iters: 5,
        timeout_ms: 5,
        ..options.clone()
    };
    let _results = assert2_msg_matches!(alex, billy, expected, fast_timeout_options);

    // Billy asks for that entry
    request_entry_ok(billy, &entry, options);
}

/// Return lots of entries
pub fn many_aspects_test(alex: &mut NodeMock, billy: &mut NodeMock, options: &ProcessingOptions) {
    // Author & hold several aspects on same address
    alex.author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
        .unwrap();
    alex.author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_2.clone()], false)
        .unwrap();
    alex.hold_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_3.clone()])
        .unwrap();
    let entry_2 = alex
        .hold_entry(&ENTRY_ADDRESS_2, vec![ASPECT_CONTENT_4.clone()])
        .unwrap();
    println!("\nAlex authored and stored Aspects \n");

    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), provider_agent_id: AgentPubKey\\(HashString\\(\"billy\"\\)\\), entry_address: EntryHash\\(HashString\\(\"entry_addr_1\"\\)\\), entry_aspect: EntryAspectData \\{ aspect_address: AspectHash\\(HashString\\(\"[\\w\\d]+\"\\)\\), type_hint: \"NodeMock\", aspect: \"[\\w\\d\\-]+\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, billy, expected, options);

    // TODO figure out something to explicit wait for
    wait_engine_wrapper_until_no_work!(alex);
    wait_engine_wrapper_until_no_work!(billy);

    // Send AuthoringList
    // ==================
    println!("\nAlex sends AuthoringEntryList\n");
    alex.reply_to_first_HandleGetAuthoringEntryList();

    // Should receive a HandleFetchEntry request from network module after receiving authoring list
    let expected = "HandleFetchEntry\\(FetchEntryData \\{ space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), entry_address: EntryHash\\(HashString\\(\"entry_addr_1\"\\)\\), request_id: \"[\\w\\d_~]+\", provider_agent_id: AgentPubKey\\(HashString\\(\"alex\"\\)\\), aspect_address_list: None \\}\\)";
    let results = assert2_msg_matches!(alex, billy, expected, options);
    let fetch_event = &results[0].events[0];
    // extract msg data
    let fetch_data = unwrap_to!(fetch_event => Lib3hServerProtocol::HandleFetchEntry);
    trace!("fetch_data: {:?}", fetch_data);
    // Respond
    alex.reply_to_HandleFetchEntry(&fetch_data)
        .expect("Reply to HandleFetchEntry should work");

    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), provider_agent_id: AgentPubKey\\(HashString\\(\"billy\"\\)\\), entry_address: EntryHash\\(HashString\\(\"entry_addr_1\"\\)\\), entry_aspect: EntryAspectData \\{ aspect_address: AspectHash\\(HashString\\(\"[\\w\\d]+\"\\)\\), type_hint: \"NodeMock\", aspect: \"hello-1\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, billy, expected, options);
    let mut entry = billy.get_entry(&ENTRY_ADDRESS_1).unwrap();
    entry.aspect_list.sort();
    assert_eq!(entry.aspect_list.len(), 3);

    // Send GossipingList
    // ==================
    trace!("\nAlex sends GossipingEntryList\n");
    alex.reply_to_first_HandleGetGossipingEntryList();

    // Should receive a HandleFetchEntry request from network module after receiving list
    let expected = "HandleFetchEntry\\(FetchEntryData \\{ space_address: SpaceHash\\(HashString\\(\"appA\"\\)\\), entry_address: EntryHash\\(HashString\\(\"entry_addr_2\"\\)\\), request_id: \"[\\w\\d_~]+\", provider_agent_id: AgentPubKey\\(HashString\\(\"alex\"\\)\\), aspect_address_list: None \\}\\)";
    let results = assert2_msg_matches!(alex, billy, expected, options);
    trace!("results: {:?}", results);
    // Get FetchEntryData for ENTRY_ADDRESS_2
    let mut maybe_fetch_data = None;
    for process_result in results {
        for event in process_result.events {
            let temp_fetch_data = unwrap_to!(event => Lib3hServerProtocol::HandleFetchEntry);
            if temp_fetch_data.entry_address == *ENTRY_ADDRESS_2 {
                maybe_fetch_data = Some(temp_fetch_data.clone());
                break;
            }
        }
    }
    let fetch_data = maybe_fetch_data.unwrap();
    debug!("fetch_data: {:?}", fetch_data);

    // #fullsync - mirrorDht will ask for content right away
    // Respond to fetch
    debug!("Respond to fetch... ");
    alex.reply_to_HandleFetchEntry(&fetch_data)
        .expect("Reply to HandleFetchEntry should work");
    debug!("Waiting for HandleStoreEntryAspect... ");
    // Expect HandleStoreEntryAspect from receiving entry via gossip
    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), provider_agent_id: AgentPubKey\\(HashString\\(\"billy\"\\)\\), entry_address: EntryHash\\(HashString\\(\"entry_addr_2\"\\)\\), entry_aspect: EntryAspectData \\{ aspect_address: AspectHash\\(HashString\\(\"[\\w\\d]+\"\\)\\), type_hint: \"NodeMock\", aspect: \"other-4\", publish_ts: \\d+ \\} \\}\\)";
    let _results = assert2_msg_matches!(alex, billy, expected, options);

    // Billy asks for that entry
    request_entry_ok(billy, &entry_2, options);
}

use crate::{
    node_mock::NodeMock,
    test_suites::two_basic::{request_entry_1, TwoNodesTestFn},
    utils::constants::*,
};
use lib3h_protocol::protocol_server::Lib3hServerProtocol;

lazy_static! {
    pub static ref TWO_NODES_GET_LISTS_TEST_FNS: Vec<(TwoNodesTestFn, bool)> = vec![
        (author_list_test, true),
        (hold_list_test, true),
        (empty_author_list_test, true),
    ];
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

/// Return some entry in authoring_list request
pub fn author_list_test(alex: &mut NodeMock, billy: &mut NodeMock) {
    // author an entry without publishing it
    alex.author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], false)
        .unwrap();
    // Reply to the publish_list request received from network module
    alex.reply_to_first_HandleGetAuthoringEntryList();
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
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1, "{:?}", srv_msg_list);

    // Billy asks for that entry
    request_entry_1(billy);
}

/// Return some entry in gossiping_list request
pub fn hold_list_test(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Have alex hold some data
    alex.hold_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], false)
        .unwrap();
    // Alex: Look for the hold_list request received from network module and reply
    alex.reply_to_first_HandleGetGossipingEntryList();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);

    // #fullsync
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
    let (did_work, _srv_msg_list) = billy.process().unwrap();
    assert!(did_work);

    // Billy asks for that entry
    request_entry_1(billy);
}

///
pub fn empty_author_list_test(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Alex replies an empty list to the initial HandleGetAuthoringEntryList
    alex.reply_to_first_HandleGetAuthoringEntryList();
    let (did_work, _srv_msg_list) = alex.process().unwrap();
    assert!(did_work);

    // Billy asks for unpublished data.
    println!("\n{} requesting entry: ENTRY_ADDRESS_1\n", billy.name);
    let query_data = billy.request_entry(ENTRY_ADDRESS_1.clone());
    let (did_work, _srv_msg_list) = billy.process().unwrap();
    assert!(did_work);

    // #fullsync
    // Alex sends back a failureResult response to the network
    println!("\n{} looking for HandleQueryEntry\n", billy.name);
    let res = billy.reply_to_HandleQueryEntry(&query_data);
    println!("\n{} found: {:?}\n", billy.name, res);
    assert!(res.is_err());
    let result_data = res.err().unwrap();
    let info = std::string::String::from_utf8_lossy(&result_data.result_info).to_string();
    assert_eq!(info, "No entry found");
}

use crate::{
    node_mock::NodeMock,
    test_suites::two_basic::{request_entry_ok, TwoNodesTestFn},
    utils::constants::*,
};
use lib3h_protocol::{data_types::EntryData, protocol_server::Lib3hServerProtocol};
use rmp_serde::Deserializer;
use serde::Deserialize;

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
pub fn author_list_test(alex: &mut NodeMock, billy: &mut NodeMock) {
    // author an entry without publishing it
    let entry = alex
        .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], false)
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
    request_entry_ok(billy, &entry);
}

/// Return some entry in gossiping_list request
pub fn hold_list_test(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Have alex hold some data
    let entry = alex
        .hold_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], false)
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
    request_entry_ok(billy, &entry);
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

/// Return author_list with already known entry
pub fn author_list_known_entry_test(alex: &mut NodeMock, billy: &mut NodeMock) {
    let entry = alex
        .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
        .unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);

    alex.reply_to_first_HandleGetAuthoringEntryList();
    let (did_work, _srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
    let (did_work, _srv_msg_list) = billy.process().unwrap();
    assert!(did_work);

    // Billy asks for that entry
    request_entry_ok(billy, &entry);
}

/// Return lots of entries
pub fn many_aspects_test(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Author & hold several aspects on same address
    alex.author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
        .unwrap();
    alex.author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_2.clone()], false)
        .unwrap();
    alex.hold_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_3.clone()], false)
        .unwrap();
    println!("\nAlex authored and stored Aspects \n");
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);

    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
    println!("\nBilly should receive first aspect \n");
    let (did_work, srv_msg_list) = billy.process().unwrap();
    println!("\nBilly srv_msg_list = {:?}\n", srv_msg_list);
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let _ = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::HandleStoreEntryAspect);

    // Send AuthoringList
    // ==================
    println!("\nAlex sends AuthoringEntryList\n");
    alex.reply_to_first_HandleGetAuthoringEntryList();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    println!("\nAlex srv_msg_list = {:?}\n", srv_msg_list);
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

    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
    let (did_work, srv_msg_list) = billy.process().unwrap();
    println!("\nBilly srv_msg_list = {:?}\n", srv_msg_list);
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 4);
    //let _ = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::HandleFetchEntry); // #fullsync
    let _ = unwrap_to!(srv_msg_list[1] => Lib3hServerProtocol::HandleStoreEntryAspect);
    let _ = unwrap_to!(srv_msg_list[2] => Lib3hServerProtocol::HandleStoreEntryAspect);
    let _ = unwrap_to!(srv_msg_list[3] => Lib3hServerProtocol::HandleStoreEntryAspect);

    // Send GossipingList
    // ==================
    // Send HoldingEntryList and should receive a HandleFetchEntry request from network module
    println!("\nAlex sends GossipingEntryList\n");
    alex.reply_to_first_HandleGetGossipingEntryList();
    let (did_work, _srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    // Process the HoldEntry generated from receiving the HandleStoreEntryAspect
    let (did_work, _srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    println!("\nBilly srv_msg_list = {:?}\n", srv_msg_list);

    // Billy asks for the entry
    // ========================
    println!("\n{} requesting entry: ENTRY_ADDRESS_1\n", billy.name);
    let query_data = billy.request_entry(ENTRY_ADDRESS_1.clone());
    let (did_work, _srv_msg_list) = billy.process().unwrap();
    assert!(did_work);

    // #fullsync
    // Billy sends that data back to the network
    println!("\n{} reply to own request:\n", billy.name);
    let _ = billy.reply_to_HandleQueryEntry(&query_data).unwrap();
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1, "{:?}", srv_msg_list);
    let msg = unwrap_to!(srv_msg_list[0] => Lib3hServerProtocol::QueryEntryResult);
    assert_eq!(&msg.entry_address, &*ENTRY_ADDRESS_1);
    let mut de = Deserializer::new(&msg.query_result[..]);
    let maybe_entry: Result<EntryData, rmp_serde::decode::Error> =
        Deserialize::deserialize(&mut de);
    let query_result = maybe_entry.unwrap();
    assert_eq!(query_result.entry_address, ENTRY_ADDRESS_1.clone());
    assert_eq!(query_result.aspect_list.len(), 3);
    assert!(
        query_result.aspect_list[0].aspect_address.clone() == *ASPECT_ADDRESS_1
            || query_result.aspect_list[0].aspect_address.clone() == *ASPECT_ADDRESS_2
            || query_result.aspect_list[0].aspect_address.clone() == *ASPECT_ADDRESS_3
    );
    assert!(
        query_result.aspect_list[1].aspect_address.clone() == *ASPECT_ADDRESS_1
            || query_result.aspect_list[1].aspect_address.clone() == *ASPECT_ADDRESS_2
            || query_result.aspect_list[1].aspect_address.clone() == *ASPECT_ADDRESS_3
    );
    assert!(
        query_result.aspect_list[2].aspect_address.clone() == *ASPECT_ADDRESS_1
            || query_result.aspect_list[2].aspect_address.clone() == *ASPECT_ADDRESS_2
            || query_result.aspect_list[2].aspect_address.clone() == *ASPECT_ADDRESS_3
    );
}

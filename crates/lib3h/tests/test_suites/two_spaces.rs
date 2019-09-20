use crate::{
    node_mock::NodeMock,
    test_suites::two_basic::{
        test_author_one_aspect, test_send_message, two_join_space, TwoNodesTestFn,
    },
    utils::constants::*,
};
use lib3h_protocol::protocol_server::Lib3hServerProtocol;

lazy_static! {
    pub static ref TWO_NODES_SPACES_TEST_FNS: Vec<(TwoNodesTestFn, bool)> = vec![
        (test_leave_space, true),
        (test_rejoining, true),
        (test_multispace_send, true),
        (test_multispace_dht, true),
    ];
}

/// Sending a Message before doing a 'TrackDna' should fail
pub fn test_leave_space(alex: &mut NodeMock, billy: &mut NodeMock) {
    // LeaveSpace
    let req_id = alex
        .leave_current_space()
        .expect("Failed sending LeaveSpace message on alex");
    assert_process_success!(alex, req_id);
    alex.set_current_space(&SPACE_ADDRESS_A);

    // Send a message from Alex to Billy
    // =================================
    println!("\n Alex trying to send DirectMessage...\n");
    alex.send_direct_message(&BILLY_AGENT_ID, ASPECT_CONTENT_1.clone());
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    println!("response: {:?}", srv_msg_list);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::FailureResult(response) = msg_1 {
        let content = std::str::from_utf8(response.result_info.as_slice()).unwrap();
        assert_eq!(content, "Unknown error encountered: \'No space at chainId\'.");
    });
    // Billy should not receive it.
    let res = billy.wait_with_timeout(
        Box::new(one_is!(Lib3hServerProtocol::HandleSendDirectMessage(_))),
        1000,
    );
    assert!(res.is_none());

    // Send a message from Billy to Alex
    // =================================
    println!("\n Billy trying to send DirectMessage...\n");
    let _req_id = billy.send_direct_message(&ALEX_AGENT_ID, ASPECT_CONTENT_1.clone());
    wait_engine_wrapper_did_work!(billy);
    // Alex should not receive it.
    let res = alex.wait_with_timeout(
        Box::new(one_is!(Lib3hServerProtocol::HandleSendDirectMessage(_))),
        1000,
    );
    assert!(res.is_none());

    // Alex sends a message to self
    // ============================
    let req_id = alex.send_direct_message(&ALEX_AGENT_ID, ASPECT_CONTENT_1.clone());
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::FailureResult(response) = msg_1 {
        assert_eq!(response.request_id, req_id);
    });
}

/// Sending a Message before doing a 'TrackDna' should fail
pub fn test_rejoining(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Alex LeaveSpace
    let req_id = alex
        .leave_current_space()
        .expect("Failed sending LeaveSpace message on Alex");
    assert_process_success!(alex, req_id);
    // Billy LeaveSpace
    let req_id = billy
        .leave_current_space()
        .expect("Failed sending LeaveSpace message on Billy");
    assert_process_success!(billy, req_id);
    // Alex and Billy re-joins
    println!("\nAlex and Billy re-joins...\n");
    two_join_space(alex, billy, &SPACE_ADDRESS_A);
    // Do some test
    println!("\nTest send DirectMessage...\n");
    test_send_message(alex, billy);

    // Alex re-joins again
    println!("\nAlex re-joins again...\n");
    let req_id = alex.join_space(&SPACE_ADDRESS_A.clone(), true).unwrap();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::FailureResult(response) = msg_1 {
        assert_eq!(response.request_id, req_id);
    });
}

/// Sending a Message before doing a 'TrackDna' should fail
pub fn test_multispace_send(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Alex LeaveSpace
    let req_id = alex
        .leave_current_space()
        .expect("Failed sending LeaveSpace message on Alex");
    assert_process_success!(alex, req_id);
    // Alex and Billy joins other spaces
    println!("\nAlex and Billy re-joins...\n");
    two_join_space(alex, billy, &SPACE_ADDRESS_B);
    two_join_space(alex, billy, &SPACE_ADDRESS_C);

    // Send messages on SPACE B
    // ========================
    println!("\nTest send DirectMessage in space B...\n");
    alex.set_current_space(&SPACE_ADDRESS_B);
    billy.set_current_space(&SPACE_ADDRESS_B);
    test_send_message(alex, billy);

    // Send messages on SPACE C
    // ========================
    println!("\nTest send DirectMessage in space C...\n");
    alex.set_current_space(&SPACE_ADDRESS_C);
    billy.set_current_space(&SPACE_ADDRESS_C);
    test_send_message(alex, billy);

    // Send messages on SPACE A - should fail
    // ========================
    println!("\nTest send DirectMessage in space A...\n");
    alex.set_current_space(&SPACE_ADDRESS_A);
    billy.set_current_space(&SPACE_ADDRESS_A);
    let req_id = alex.send_direct_message(&CAMILLE_AGENT_ID, "marco".as_bytes().to_vec());
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    println!("response: {:?}", srv_msg_list);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::FailureResult(response) = msg_1 {
        assert_eq!(response.request_id, req_id);
    });
}

/// Sending a Message before doing a 'TrackDna' should fail
pub fn test_multispace_dht(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Alex LeaveSpace
    let req_id = alex
        .leave_current_space()
        .expect("Failed sending LeaveSpace message on Alex");
    assert_process_success!(alex, req_id);
    // Alex and Billy joins other spaces
    println!("\nAlex and Billy re-joins...\n");
    two_join_space(alex, billy, &SPACE_ADDRESS_B);
    two_join_space(alex, billy, &SPACE_ADDRESS_C);

    // Author entry on SPACE B
    // =======================
    println!("\nTest send DirectMessage in space B...\n");
    alex.set_current_space(&SPACE_ADDRESS_B);
    billy.set_current_space(&SPACE_ADDRESS_B);
    test_author_one_aspect(alex, billy);

    // Author entry on SPACE C
    // =======================
    println!("\nTest send DirectMessage in space C...\n");
    alex.set_current_space(&SPACE_ADDRESS_C);
    billy.set_current_space(&SPACE_ADDRESS_C);
    test_author_one_aspect(alex, billy);
}

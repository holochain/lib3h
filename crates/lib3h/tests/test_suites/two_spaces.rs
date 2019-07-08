use crate::{node_mock::NodeMock, test_suites::two_basic::TwoNodesTestFn, utils::constants::*};
use lib3h_protocol::protocol_server::Lib3hServerProtocol;

lazy_static! {
    pub static ref TWO_NODES_SPACES_TEST_FNS: Vec<(TwoNodesTestFn, bool)> =
        vec![(test_leave_space, true),];
}

/// Sending a Message before doing a 'TrackDna' should fail
pub fn test_leave_space(alex: &mut NodeMock, billy: &mut NodeMock) {
    // LeaveSpace
    let req_id = alex
        .leave_current_space()
        .expect("Failed sending LeaveSpace message on alex");
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    println!("response: {:?}", srv_msg_list);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::SuccessResult(response) = msg_1 {
        assert_eq!(response.request_id, req_id);
    });
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
        assert_eq!(content, "Agent alex does not track space SPACE_A");
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
    let req_id = billy.send_direct_message(&ALEX_AGENT_ID, ASPECT_CONTENT_1.clone());
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::SuccessResult(response) = msg_1 {
        assert_eq!(response.request_id, req_id);
    });

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

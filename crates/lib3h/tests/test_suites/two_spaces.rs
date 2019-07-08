use crate::{
    node_mock::NodeMock,
    test_suites::two_basic::{request_entry_ok, TwoNodesTestFn},
    utils::constants::*,
};
use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol};
use rmp_serde::Deserializer;
use serde::Deserialize;

lazy_static! {
    pub static ref TWO_NODES_SPACES_TEST_FNS: Vec<(TwoNodesTestFn, bool)> =
        vec![(test_untrack_alex, true),];
}

/// Sending a Message before doing a 'TrackDna' should fail
pub fn test_untrack_alex(alex: &mut NodeMock, billy: &mut NodeMock) {
    // Send LeaveSpace
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

    // Send a message from alex to billy
    println!("\nTrying to send DirectMessage...\n");
    let before_count = alex.count_recv_messages();
    alex.send_direct_message(&BILLY_AGENT_ID, ASPECT_CONTENT_1.clone());
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert!(did_work);
    // assert_eq!(srv_msg_list.len(), 1);
    println!("response: {:?}", srv_msg_list);
    // Billy should not receive it.
    let res = billy.wait_with_timeout(
        Box::new(one_is!(Lib3hServerProtocol::HandleSendDirectMessage(_))),
        1000,
    );
    assert!(res.is_none());
}

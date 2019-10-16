use crate::{
    node_mock::*,
    test_suites::two_basic::{test_author_one_aspect, test_send_message, TwoNodesTestFn},
    utils::processor_harness::*,
};
use lib3h_protocol::protocol_server::Lib3hServerProtocol;

lazy_static! {
    pub static ref TWO_NODES_CONNECTION_TEST_FNS: Vec<(TwoNodesTestFn, bool)> = vec![
        // TODO Issue #236
        (test_two_disconnect, true),
        (test_two_gossip_self, true),
        (test_two_peer_timeout, true),
        // TODO Issue #236
        (test_two_peer_timeout_reconnect, true),
        (test_two_reconnect, true),
    ];
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

/// Have Alex disconnect and reconnect TODO Issue #236
fn test_two_disconnect(alex: &mut NodeMock, billy: &mut NodeMock, _options: &ProcessingOptions) {
    alex.disconnect();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert_eq!(srv_msg_list.len(), 0);
    println!("srv_msg_list = {:?} ({})\n", srv_msg_list, did_work);
    // Should be disconnected from the network
    let (did_work, srv_msg_list) = billy.process().unwrap();
    println!("srv_msg_list = {:?} ({})\n", srv_msg_list, did_work);
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::Disconnected(response) = msg_1 {
        assert_eq!(response.network_id, "FIXME");
    });
}

/// Wait for peer timeout
fn test_two_gossip_self(alex: &mut NodeMock, billy: &mut NodeMock, _options: &ProcessingOptions) {
    // Wait before peer Timeout threshold
    // TODO make a faster way to test this, such as reducing the time out value in the config
    std::thread::sleep(std::time::Duration::from_millis(1000));
    // Billy should send a PeerTimedOut message
    let (did_work, srv_msg_list) = billy.process().unwrap();
    println!("srv_msg_list = {:?} ({})\n", srv_msg_list, did_work);
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 0);

    // Alex should gossip self
    let (did_work, srv_msg_list) = alex.process().unwrap();
    println!("srv_msg_list = {:?} ({})\n", srv_msg_list, did_work);
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 0);

    // More process: Have Billy process P2p::PeerName of alex
    let (_did_work, _srv_msg_list) = billy.process().unwrap();
    let (_did_work, _srv_msg_list) = alex.process().unwrap();

    // Wait past peer Timeout threshold
    // TODO make a faster way to test this
    std::thread::sleep(std::time::Duration::from_millis(2100));
    // Billy should not see a PeerTimedOut message
    let (did_work, srv_msg_list) = billy.process().unwrap();
    println!("srv_msg_list = {:?} ({})\n", srv_msg_list, did_work);
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 0);
}

/// Wait for peer timeout
#[allow(dead_code)]
fn test_two_peer_timeout(alex: &mut NodeMock, billy: &mut NodeMock, options: &ProcessingOptions) {
    // Wait before peer Timeout threshold
    std::thread::sleep(std::time::Duration::from_millis(1000));
    // Billy should NOT send a PeerTimedOut message
    let (did_work, srv_msg_list) = billy.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 0);
    // Wait past peer Timeout threshold
    std::thread::sleep(std::time::Duration::from_millis(2100));
    // Billy SHOULD send a PeerTimedOut message ...

    let equal_to =
        Lib3hServerProtocol::Disconnected(lib3h_protocol::data_types::DisconnectedData {
            network_id: "FIXME".into(), // TODO
        });

    assert2_msg_eq!(alex, billy, equal_to, options);
}

/// Wait for peer timeout than reconnect
#[allow(dead_code)]
fn test_two_peer_timeout_reconnect(
    alex: &mut NodeMock,
    billy: &mut NodeMock,
    options: &ProcessingOptions,
) {
    // Wait past peer Timeout threshold
    // TODO make this timeout much faster or mock time
    std::thread::sleep(std::time::Duration::from_millis(3100));
    let disconnect1 = Box::new(Lib3hServerProtocolEquals(
        Lib3hServerProtocol::Disconnected(lib3h_protocol::data_types::DisconnectedData {
            network_id: "FIXME".into(), // TODO
        }),
    ));
    let disconnect2 = Box::new(Lib3hServerProtocolEquals(
        Lib3hServerProtocol::Disconnected(lib3h_protocol::data_types::DisconnectedData {
            network_id: "FIXME".into(), // TODO
        }),
    ));
    assert2_processed!(billy, alex, disconnect1);
    assert2_processed!(billy, alex, disconnect2);

    println!("\n Reconnecting Alex...\n");
    let connect_data = alex.reconnect().expect("Reconnection failed");
    wait_connect!(alex, connect_data, billy);

    alex.wait_until_no_work();
    billy.wait_until_no_work();
    test_send_message(alex, billy, options);
    test_author_one_aspect(alex, billy, options);
}

/// Have Alex disconnect and reconnect
#[allow(dead_code)]
fn test_two_reconnect(alex: &mut NodeMock, billy: &mut NodeMock, options: &ProcessingOptions) {
    alex.disconnect();
    let (did_work, srv_msg_list) = alex.process().unwrap();
    assert_eq!(srv_msg_list.len(), 0);
    println!(
        "disconnect srv_msg_list = {:?} ({})\n",
        srv_msg_list, did_work
    );
    // Should be disconnected from the network
    let (did_work, srv_msg_list) = billy.process().unwrap();
    println!("srv_msg_list = {:?} ({})\n", srv_msg_list, did_work);
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 1);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::Disconnected(response) = msg_1 {
        assert_eq!(response.network_id, "FIXME");
    });

    println!("\n Reconnecting Alex...\n");
    let connect_data = alex.reconnect().expect("Reconnection failed");

    wait_connect!(billy, connect_data, alex);

    // TODO wait for an explicit message instead
    alex.wait_until_no_work();
    billy.wait_until_no_work();
    alex.wait_until_no_work();

    test_send_message(alex, billy, options);
    test_author_one_aspect(alex, billy, options);
}

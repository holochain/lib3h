use crate::{node_mock::NodeMock, utils::constants::*};
use lib3h_protocol::protocol_server::Lib3hServerProtocol;

pub type MultiNodeTestFn = fn(nodes: &mut Vec<NodeMock>);

lazy_static! {
    pub static ref MIRROR_TEST_FNS: Vec<(MultiNodeTestFn, bool)> =
        vec![(test_setup_only, true), (test_mirror, true),];
}

//--------------------------------------------------------------------------------------------------
// Test setup
//--------------------------------------------------------------------------------------------------

#[allow(dead_code)]
pub fn setup_mirror_nodes(nodes: &mut Vec<NodeMock>) {
    let space_address = &SPACE_ADDRESS_A;
    for node in nodes {
        println!("\n {} joins {:?}\n", node.name(), *SPACE_ADDRESS_A);
        let req_id = node.join_space(&space_address, true).unwrap();
        let (did_work, srv_msg_list) = node.process().unwrap();
        assert!(did_work);
        assert_eq!(srv_msg_list.len(), 3);
        let msg_1 = &srv_msg_list[0];
        one_let!(Lib3hServerProtocol::SuccessResult(response) = msg_1 {
            assert_eq!(response.request_id, req_id);
        });
        wait_engine_wrapper_until_no_work!(node);
    }
    println!(
        "DONE setup_mirror_nodes() DONE \n\n =================================================\n"
    );
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

/// Empty function that triggers the test suite
#[allow(dead_code)]
fn test_setup_only(_nodes: &mut Vec<NodeMock>) {
    // n/a
}

fn test_mirror(nodes: &mut Vec<NodeMock>) {
    let mut node1 = nodes.pop().unwrap();
    let mut node2 = nodes.pop().unwrap();
    {
        // node1 publishes data on the network
        let _entry = node1
            .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
            .unwrap();
    }
    let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: HashString\\(\"\\w+\"\\), provider_agent_id: HashString\\(\"mirror_node9\"\\), entry_address: HashString\\(\"entry_addr_1\"\\), entry_aspect: EntryAspectData \\{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"hello-1\", publish_ts: \\d+ \\} \\}\\)";

    let _results = assert2_msg_matches!(node1, node2, expected);
    nodes.push(node2);
    nodes.push(node1);

    // TODO: the rest of the code that proves out mirroring
}

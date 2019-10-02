use crate::{
    node_mock::{test_join_space, NodeMock},
    utils::constants::*,
};

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
    nodes_join_space(nodes);
    for node in nodes {
        wait_engine_wrapper_until_no_work!(node);
    }
    println!(
        "DONE setup_mirror_nodes() DONE \n\n =================================================\n"
    );
}

fn nodes_join_space(nodes: &mut Vec<NodeMock>) {
    for node in nodes {
        test_join_space(node, &SPACE_ADDRESS_A);
    }
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
    let entry = {
        let mut node1 = nodes.pop().unwrap();
        let mut node2 = nodes.pop().unwrap();

        // node1 publishes data on the network
        let entry = node1
            .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
            .unwrap();

        let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: HashString\\(\"\\w+\"\\), provider_agent_id: HashString\\(\"mirror_node9\"\\), entry_address: HashString\\(\"entry_addr_1\"\\), entry_aspect: EntryAspectData \\{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"hello-1\", publish_ts: \\d+ \\} \\}\\)";

        let _results = assert2_msg_matches!(node1, node2, expected);

        assert_eq!(entry, node1.get_entry(&ENTRY_ADDRESS_1).unwrap());
        nodes.push(node2);
        nodes.push(node1);
        entry
    };

    for _i in 0..20 {
        process_nodes(nodes);
    }

    for node in nodes {
        assert_eq!(entry, node.get_entry(&ENTRY_ADDRESS_1).unwrap());
    }
}

fn process_nodes(nodes: &mut Vec<NodeMock>) {
    for node in nodes {
        let _result = node.process();
    }
}

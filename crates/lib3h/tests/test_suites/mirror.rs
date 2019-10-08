use crate::{
    node_mock::{test_join_space, NodeMock},
    utils::{constants::*, processor_harness::ProcessingOptions},
};

pub type MultiNodeTestFn = fn(nodes: &mut Vec<NodeMock>, options: &ProcessingOptions);

lazy_static! {
    pub static ref MIRROR_TEST_FNS: Vec<(MultiNodeTestFn, bool)> = vec![
        (test_setup_only, true),
        (test_mirror_from_center, true),
        (test_mirror_from_edge, true),
    ];
}

//--------------------------------------------------------------------------------------------------
// Test setup
//--------------------------------------------------------------------------------------------------

#[allow(dead_code)]
pub fn setup_mirror_nodes(nodes: &mut Vec<NodeMock>, options: &ProcessingOptions) {
    // Wait for nodes to auto-connect with Discovery
    assert!(nodes.len() > 1);
    let mut node0 = nodes.remove(0);
    let mut node1 = nodes.remove(0);
    let expected = "Connected\\(ConnectedData \\{ request_id: \"[\\w\\d_~]+\", uri: Lib3hUri\\(\".*\"\\) \\}\\)";
    let _results = assert_msg_matches!(node1, expected, options);
    wait_engine_wrapper_until_no_work!(node0);
    nodes.insert(0, node1);
    nodes.insert(0, node0);

    nodes_join_space(nodes, options);
    process_nodes(nodes, options);
    debug!(
        "DONE setup_mirror_nodes() DONE \n\n =================================================\n"
    );
}

fn nodes_join_space(nodes: &mut Vec<NodeMock>, options: &ProcessingOptions) {
    for node in nodes {
        test_join_space(node, &SPACE_ADDRESS_A, options);
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

/// Empty function that triggers the test suite
#[allow(dead_code)]
fn test_setup_only(_nodes: &mut Vec<NodeMock>, _options: &ProcessingOptions) {
    // n/a
}

// test using node0, the one all the other nodes connected to
// as the publisher of the entry
fn test_mirror_from_center(nodes: &mut Vec<NodeMock>, options: &ProcessingOptions) {
    let entry = {
        let mut node0 = nodes.remove(0);
        let mut node1 = nodes.remove(0);
        // node0 publishes data on the network
        let agent_id = node0.agent_id();

        let entry = node0
            .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
            .unwrap();

        trace!("[test_mirror_from_center] node0: {}", node0.name());
        let expected = format!("HandleStoreEntryAspect\\(StoreEntryAspectData \\{{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), provider_agent_id: AgentPubKey\\(HashString\\(\"{}\"\\)\\), entry_address: HashString\\(\"entry_addr_1\"\\), entry_aspect: EntryAspectData \\{{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"hello-1\", publish_ts: \\d+ \\}} \\}}\\)", agent_id);

        let _results = assert2_msg_matches!(node0, node1, expected.as_str(), options);

        assert_eq!(entry, node0.get_entry(&ENTRY_ADDRESS_1).unwrap());
        nodes.insert(0, node1);
        nodes.insert(0, node0);
        entry
    };

    let mut checked = std::collections::HashSet::new();

    let clock = std::time::SystemTime::now();
    let timeout = std::time::Duration::from_millis(60000);
    let max_iters = 100000;
    let delay_interval = std::time::Duration::from_millis(options.delay_interval_ms);
    for _i in 0..max_iters {
        process_nodes(nodes, options);

        check_entries(nodes, &mut checked, &entry);
        if checked.len() == nodes.len() {
            trace!("Mirror entry check found all nodes.");
            break;
        }
        let elapsed = clock.elapsed().unwrap();
        if elapsed > timeout {
            trace!("Mirror entry check timeout");
            break;
        }
        std::thread::sleep(delay_interval);
    }
    assert!(checked.len() == nodes.len());
}

fn check_entries(
    nodes: &mut Vec<NodeMock>,
    checked: &mut std::collections::HashSet<String>,
    entry: &lib3h_protocol::data_types::EntryData,
) {
    for node in nodes {
        if checked.contains(&node.name()) {
            continue;
        }

        trace!("checking if {} has entry...", node.name());
        let last = node.get_entry(&ENTRY_ADDRESS_1);
        if let Some(entry2) = last.clone() {
            if &entry2 == entry {
                trace!("Found entry for node {}", node.name());
                checked.insert(node.name());
            }
        }
    }
}

// test using nodeN, NOT the one all the other nodes connected to
// as the publisher of the entry
fn test_mirror_from_edge(nodes: &mut Vec<NodeMock>, options: &ProcessingOptions) {
    let entry = {
        let mut node0 = nodes.remove(0);
        let mut noden = nodes.pop().unwrap();
        // node0 publishes data on the network
        let entry = noden
            .author_entry(&ENTRY_ADDRESS_1, vec![ASPECT_CONTENT_1.clone()], true)
            .unwrap();

        let expected = "HandleStoreEntryAspect\\(StoreEntryAspectData \\{ request_id: \"[\\w\\d_~]+\", space_address: SpaceHash\\(HashString\\(\"\\w+\"\\)\\), provider_agent_id: HashString\\(\"mirror_node0\"\\), entry_address: HashString\\(\"entry_addr_1\"\\), entry_aspect: EntryAspectData \\{ aspect_address: HashString\\(\"[\\w\\d]+\"\\), type_hint: \"NodeMock\", aspect: \"hello-1\", publish_ts: \\d+ \\} \\}\\)";

        let _results = assert2_msg_matches!(node0, noden, expected, options);

        assert_eq!(entry, node0.get_entry(&ENTRY_ADDRESS_1).unwrap());
        nodes.push(noden);
        nodes.insert(0, node0);
        entry
    };

    process_nodes(nodes, options);

    for node in nodes {
        println!("checking if {} has entry...", node.name());
        assert_eq!(entry, node.get_entry(&ENTRY_ADDRESS_1).unwrap());
        println!("yes!");
    }
}

fn process_nodes(nodes: &mut Vec<NodeMock>, options: &ProcessingOptions) {
    let timeout = std::time::Duration::from_millis(1000);

    let delay_interval = std::time::Duration::from_millis(1);
    let clock = std::time::SystemTime::now();
    for _i in 0..options.max_iters {
        process_nodes_inner(nodes);
        let elapsed = clock.elapsed().unwrap();
        if elapsed > timeout {
            trace!(
                "[process_nodes] timed out (elapsed={:?} ms)",
                elapsed.as_millis()
            );
            break;
        }
        std::thread::sleep(delay_interval);
    }
}

fn process_nodes_inner(nodes: &mut Vec<NodeMock>) {
    for node in nodes {
        //wait_engine_wrapper_until_no_work!(node);
        let _result = node.process();
    }
}

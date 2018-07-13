extern crate lib3h;

use std::time::Instant;

use lib3h::node::{Endpoint, Node, Event};

static NODE_A: &'static [u8] = b"node-A";
static NODE_B: &'static [u8] = b"node-B";
static NODE_C: &'static [u8] = b"node-C";

fn node_info(nodes: &mut Vec<Node>, index: usize) -> (&mut Vec<Node>, Vec<u8>, String) {
    let nid = nodes[index].get_node_id();
    let nid_disp = String::from_utf8_lossy(&nid).to_string();
    (nodes, nid, nid_disp)
}

fn main() {
    let mut nodes: Vec<Node> = Vec::new();

    let test_sequence: Vec<Box<Fn(&mut Vec<Node>)>> = vec![
        Box::new(|nodes: &mut Vec<Node>| {
            let (nodes, from_node, from_node_disp) = node_info(nodes, 0);
            let (nodes, to_node, to_node_disp) = node_info(nodes, 1);
            println!("[test] from: {}, to: {}", from_node_disp, to_node_disp);
            nodes[0].send(&to_node, b"hello");
        }),
        Box::new(|nodes: &mut Vec<Node>| {
            let (nodes, from_node, from_node_disp) = node_info(nodes, 2);
            let (nodes, to_node, to_node_disp) = node_info(nodes, 0);
            println!("[test] from: {}, to: {}", from_node_disp, to_node_disp);
            for con_id in nodes[2].get_connected_nodes() {
                println!("- (have connected): {}",
                    String::from_utf8_lossy(&con_id));
            }
            nodes[0].send(&to_node, b"hello");
        }),
        /*
        Box::new(|nodes: &mut Vec<Node>| {
            let (nodes, from_node, from_node_disp) = node_info(nodes, 1);
            let (nodes, to_node, to_node_disp) = node_info(nodes, 2);
            println!("[test] from: {}, to: {}", from_node_disp, to_node_disp);
        }),
        Box::new(|nodes: &mut Vec<Node>| {
            let (nodes, from_node, from_node_disp) = node_info(nodes, 2);
            let (nodes, to_node, to_node_disp) = node_info(nodes, 0);
            println!("[test] from: {}, to: {}", from_node_disp, to_node_disp);
        }),
        Box::new(|nodes: &mut Vec<Node>| {
            let (nodes, from_node, from_node_disp) = node_info(nodes, 2);
            let (nodes, to_node1, to_node_disp1) = node_info(nodes, 0);
            let (nodes, to_node2, to_node_disp2) = node_info(nodes, 1);
            println!("[test] from: {}, to: {}, {}", from_node_disp, to_node_disp1, to_node_disp2);
        }),
        */
    ];

    {
        let listen: Vec<Endpoint> = vec![
            Endpoint::new("127.0.0.1", 12001),
            Endpoint::new("[::1]", 12001),
        ];
        nodes.push(Node::new(NODE_A, &listen, &vec![]));
    }

    let mut all_ready = false;
    let mut last_time = Instant::now();
    let mut index = 0;
    loop {
        let mut did_something = false;

        {
            let mut new_nodes: Vec<Node> = Vec::new();
            for mut node in nodes.drain(..) {
                let nid;
                let nid_disp;
                let events;
                {
                    nid = node.get_node_id();
                    nid_disp = String::from_utf8_lossy(&nid);
                    events = node.process_once();
                }
                for event in events {
                    did_something = true;
                    println!("Got event: {:?}", event);
                    match event {
                        Event::OnReady => {
                            if nid.as_slice() == NODE_A {
                                println!("node-A Ready");
                                let listen: Vec<Endpoint> = vec![
                                    Endpoint::new("127.0.0.1", 12002),
                                    Endpoint::new("[::1]", 12002),
                                ];
                                let connect: Vec<Endpoint> = vec![
                                    Endpoint::new("127.0.0.1", 12001),
                                ];
                                new_nodes.push(Node::new(NODE_B, &listen, &connect));
                            } else if nid.as_slice() == NODE_B {
                                println!("node-B Ready");
                                let listen: Vec<Endpoint> = vec![
                                    Endpoint::new("127.0.0.1", 12003),
                                    Endpoint::new("[::1]", 12003),
                                ];
                                let connect: Vec<Endpoint> = vec![
                                    Endpoint::new("127.0.0.1", 12001),
                                    Endpoint::new("127.0.0.1", 12002),
                                ];
                                new_nodes.push(Node::new(NODE_C, &listen, &connect));
                            } else if nid.as_slice() == NODE_C {
                                println!("node-C Ready");
                                all_ready = true;
                                last_time = Instant::now();
                            }
                        }
                        Event::OnData(node_id, data) => {
                            let node_id = String::from_utf8_lossy(&node_id);
                            let data = String::from_utf8_lossy(&data);
                            println!("[{}] - Got data from [{}] - {}", nid_disp, node_id, data);
                        }
                        Event::OnError(e) => {
                        }
                    }
                }
                new_nodes.push(node);
            }
            nodes = new_nodes;
        }

        if all_ready && last_time.elapsed().as_secs() >= 1 {
            last_time = Instant::now();
            index += 1;
            let test = &test_sequence[index % test_sequence.len()];
            test(&mut nodes);
        }

        if !did_something {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

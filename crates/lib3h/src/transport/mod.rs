//! common types and traits for working with Transport instances

pub mod error;
pub mod memory_mock;
pub mod protocol;
pub mod transport_trait;

/// a connection identifier
pub type TransportId = String;
pub type TransportIdRef = str;

///
#[cfg(test)]
pub mod tests {
    #![allow(non_snake_case)]

    use crate::transport::{
        memory_mock::transport_memory, protocol::TransportEvent, transport_trait::Transport,
    };

    #[test]
    fn memory_send_test() {
        let mut node_A = transport_memory::TransportMemory::new();
        let mut node_B = transport_memory::TransportMemory::new();
        let uri_A = "mem://a";
        let uri_B = "mem://b";

        send_test(&mut node_A, &mut node_B, uri_A, uri_B);
    }

    fn send_test(
        node_A: &mut impl Transport,
        node_B: &mut impl Transport,
        uri_A: &str,
        uri_B: &str,
    ) {
        // Connect
        let actual_uri_a = node_A.bind(uri_A).unwrap();
        let actual_uri_b = node_B.bind(uri_B).unwrap();
        let idAB = node_A.connect(&actual_uri_b).unwrap();
        println!("actual_uri: {},", actual_uri_b);
        println!("idAB: {},", idAB);
        let (_did_work, _event_list) = node_A.process().unwrap();
        let (_did_work, _event_list) = node_B.process().unwrap();

        // Send A -> B
        let payload = [1, 2, 3, 4];
        node_A.send(&[&idAB], &payload).unwrap();
        let (did_work, event_list) = node_B.process().unwrap();
        assert!(did_work);
        assert_eq!(event_list.len(), 1);
        let recv_event = event_list[0].clone();
        let (recv_id, recv_payload) = match recv_event {
            TransportEvent::Received(a, b) => (a, b),
            _ => panic!("Received wrong TransportEvent type"),
        };
        assert_eq!(actual_uri_a, recv_id);
        assert_eq!(payload, recv_payload.as_slice());
        let (_did_work, _event_list) = node_A.process().unwrap();

        // Send B -> A
        let payload = [4, 2, 1, 3];
        let id_list = node_B.transport_id_list().unwrap();
        let idBA = id_list[0].clone();
        node_B.send(&[&idBA], &payload).unwrap();
        let (did_work, event_list) = node_A.process().unwrap();
        assert!(did_work);
        assert_eq!(event_list.len(), 1);
        let recv_event = event_list[0].clone();
        let (recv_id, recv_payload) = match recv_event {
            TransportEvent::Received(a, b) => (a, b),
            _ => panic!("Received wrong TransportEvent type"),
        };
        assert_eq!(actual_uri_b, recv_id);
        assert_eq!(payload, recv_payload.as_slice());
    }
}

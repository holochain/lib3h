//! common types and traits for working with Transport instances

pub mod error;
pub mod protocol;
pub mod transport_trait;
#[cfg(test)]
pub mod memory_server;
#[cfg(test)]
pub mod transport_memory;

/// a connection identifier
pub type TransportId = String;
pub type TransportIdRef = str;

///
#[cfg(test)]
pub mod tests {
    #![allow(non_snake_case)]

    use super::*;
    use crate::transport::{protocol::TransportEvent, transport_trait::Transport};

    #[test]
    fn memory_test() {
        let mut node_A = transport_memory::TransportMemory::new();
        let mut node_B = transport_memory::TransportMemory::new();
        let uri_B = "fakeUrl";

        send_test(&mut node_A, &mut node_B, uri_B);
    }

    fn send_test(node_A: &mut impl Transport, node_B: &mut impl Transport, uri_B: &str) {
        node_B.bind(uri_B).unwrap();
        let idAB = node_A.connect(uri_B).unwrap();
        let payload = [1, 2, 3, 4];
        node_A.send(&[&idAB], &payload).unwrap();

        let (did_work, event_list) = node_B.process().unwrap();
        assert!(did_work);
        assert_eq!(event_list.len(), 1);
        let recv_event = event_list[0].clone();
        let (recv_id, recv_payload) = match recv_event {
            TransportEvent::Received(a, b) => (a, b),
            _ => panic!("Receice wrong TransportEvent type"),
        };
        assert_eq!(idAB, recv_id);
        assert_eq!(payload, recv_payload.as_slice());
    }
}

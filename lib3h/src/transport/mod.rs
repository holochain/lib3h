//! common types and traits for working with Transport instances

pub mod error;
pub mod memory_server;
pub mod protocol;
pub mod transport_memory;
pub mod transport_trait;

/// a connection identifier
pub type TransportId = String;
pub type TransportIdRef = str;

///
#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn memory_test() {
        let mut node_A = transport_memory::TransportMemory::new();
        let mut node_B = transport_memory::TransportMemory::new();
        let uriB = "bidon";

        complete_test(node_A, node_B, uriB);
    }

    fn complete_test(node_A: impl T, node_B: impl T, uriB: &str) {
        node_B.bind(uriB).unwrap();
        let idAB = node_A.connect(uriB).unwrap();
        let payload = vec![1, 2, 3, 4];
        node_A.send(idAB, payload).unwrap();

        let (did_work, event_list) = node_B.process().unwrap();
        assert!(did_work);
        assert_eq!(event_list.len(), 1);
        let recv_event = event_list[0];
        let (recv_id, recv_payload) = unwrap_to!(recv_event => TransportEvent::Received);
        assert_eq!(idAB, recv_id);
        assert_eq!(payload, recv_payload);
    }
}

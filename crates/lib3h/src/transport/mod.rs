//! common types and traits for working with Transport instances
use url::Url;

pub mod error;
pub mod memory_mock;
pub mod protocol;
pub mod transport_crypto;
pub mod transport_trait;

/// a connection identifier
pub type TransportId = String;

// TODO make a struct for transport id and make these trait converters
pub fn transport_id_to_url(id: TransportId) -> Url {
    Url::parse(id.as_str()).expect("transport_id_to_url: transport id is not a well formed url")
}

pub fn url_to_transport_id(url: &Url) -> TransportId {
    url.to_string()
}

pub type TransportIdRef = str;

///
#[cfg(test)]
pub mod tests {
    #![allow(non_snake_case)]

    use crate::transport::{
        memory_mock::transport_memory, protocol::TransportEvent, transport_trait::Transport,
    };

    use url::Url;

    #[test]
    fn memory_send_test() {
        let mut node_A = transport_memory::TransportMemory::new();
        let mut node_B = transport_memory::TransportMemory::new();
        let uri_A = Url::parse("mem://a").unwrap();
        let uri_B = Url::parse("mem://b").unwrap();

        send_test(&mut node_A, &mut node_B, &uri_A, &uri_B);
    }

    #[test]
    fn wss_send_test() {
        let mut node_A = crate::transport_wss::TransportWss::with_std_tcp_stream();
        let mut node_B = crate::transport_wss::TransportWss::with_std_tcp_stream();
        let uri_A = Url::parse("wss://127.0.0.1:64529").unwrap();
        let uri_B = Url::parse("wss://127.0.0.1:64530").unwrap();

        send_test(&mut node_A, &mut node_B, &uri_A, &uri_B);
    }

    fn send_test(
        node_A: &mut impl Transport,
        node_B: &mut impl Transport,
        uri_A: &Url,
        uri_B: &Url,
    ) {
        // Connect
        let actual_uri_a = node_A.bind(uri_A).unwrap();
        let actual_uri_b = node_B.bind(uri_B).unwrap();
        let idAB = node_A.connect(&actual_uri_b).unwrap();
        println!("actual_uri: {}, idAB: {}", actual_uri_b, idAB);

        println!("calling node A process 1");
        let (_did_work, _event_list) = node_A.process().unwrap();

        println!("calling node B process 1");
        let (_did_work, _event_list) = node_B.process().unwrap();

        // Send A -> B
        let payload = [1, 2, 3, 4];
        node_A.send(&[&idAB], &payload).unwrap();
        println!("calling node B process 2");
        let (did_work, event_list) = node_B.process().unwrap();
        assert!(did_work);
        assert_eq!(event_list.len(), 1);
        let recv_event = event_list[0].clone();
        let (recv_id, recv_payload) = match recv_event {
            TransportEvent::Received(a, b) => (a, b),
            _ => panic!("Received wrong TransportEvent type"),
        };
        assert_eq!(actual_uri_a.as_str(), recv_id);
        assert_eq!(payload, recv_payload.as_slice());
        println!("calling node A process 2");
        let (_did_work, _event_list) = node_A.process().unwrap();

        // Send B -> A
        let payload = [4, 2, 1, 3];
        let id_list = node_B.transport_id_list().unwrap();
        let idBA = id_list[0].clone();
        node_B.send(&[&idBA], &payload).unwrap();
        println!("calling node A process 3");
        let (did_work, event_list) = node_A.process().unwrap();
        assert!(did_work);
        assert_eq!(event_list.len(), 1);
        let recv_event = event_list[0].clone();
        let (recv_id, recv_payload) = match recv_event {
            TransportEvent::Received(a, b) => (a, b),
            _ => panic!("Received wrong TransportEvent type"),
        };
        assert_eq!(actual_uri_b.as_str(), recv_id);
        assert_eq!(payload, recv_payload.as_slice());
    }
}

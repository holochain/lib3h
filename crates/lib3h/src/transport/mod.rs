//! common types and traits for working with Transport instances
use url::Url;

pub mod error;
pub mod memory_mock;
pub mod protocol;
pub mod transport_crypto;
pub mod transport_trait;

/// a connection identifier
pub type ConnectionId = String;

// TODO make a struct for transport id and make these trait converters
pub fn transport_id_to_url(id: ConnectionId) -> Url {
    // TODO this is not general enough for all transports
    Url::parse(id.as_str()).expect("transport_id_to_url: connection id is not a well formed url")
}

pub fn url_to_transport_id(url: &Url) -> ConnectionId {
    // TODO this is not general enough for all transports
    String::from(url.path())
}

pub type ConnectionIdRef = str;

///
#[cfg(test)]
pub mod tests {
    #![allow(non_snake_case)]

    use crate::transport::{
        memory_mock::transport_memory, protocol::TransportEvent, transport_trait::Transport,
    };

    use url::Url;

    // How many times to call process before checking if work was done
    const NUM_PROCESS_LOOPS: u8 = 10;

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
        let _actual_bind_uri_a = node_A.bind(uri_A).unwrap();
        let actual_bind_uri_b = node_B.bind(uri_B).unwrap();
        let idAB = node_A.connect(&actual_bind_uri_b).unwrap();
        println!("actual_uri: {}, idAB: {}", actual_bind_uri_b, idAB);

        let (_did_work, _event_list) = node_A.process().unwrap();
        let (_did_work, _event_list) = node_B.process().unwrap();

        // Send A -> B
        let payload = [1, 2, 3, 4];
        node_A.send(&[&idAB], &payload).unwrap();
        let mut did_work = false;
        let mut event_list = Vec::new();
        for _x in 0..NUM_PROCESS_LOOPS {
            let (did_work_B, mut event_list_B) = node_B.process().unwrap();
            let (_did_work_A, _event_list_A) = node_A.process().unwrap();
            event_list.append(&mut event_list_B);
            did_work |= did_work_B;
        }
        assert!(did_work);
        assert!(event_list.len() >= 1);
        let recv_event = event_list.last().unwrap().clone();
        let (recv_id, recv_payload) = match recv_event {
            TransportEvent::Received(a, b) => (a, b),
            e => panic!("Received wrong TransportEvent type: {:?}", e),
        };
        assert!(node_A.get_uri(recv_id.as_str()).is_none());
        assert!(node_B.get_uri(recv_id.as_str()).is_some());
        // TODO review this net devs
        println!(
            "node_B.get_uri({}): {}",
            recv_id,
            node_B.get_uri(recv_id.as_str()).unwrap()
        );

        assert_eq!(payload, recv_payload.as_slice());
        let (_did_work, _event_list) = node_A.process().unwrap();

        // Send B -> A
        let payload = [4, 2, 1, 3];
        let id_list = node_B.connection_id_list().unwrap();
        let idBA = id_list[0].clone();
        node_B.send(&[&idBA], &payload).unwrap();
        did_work = false;
        event_list.clear();
        for _x in 0..NUM_PROCESS_LOOPS {
            let (did_work_A, mut event_list_A) = node_A.process().unwrap();
            let (_did_work_B, _event_list_B) = node_B.process().unwrap();
            did_work |= did_work_A;
            event_list.append(&mut event_list_A);
        }
        assert!(did_work);
        assert_eq!(event_list.len(), 1);
        let recv_event = event_list[0].clone();
        let (recv_id, recv_payload) = match recv_event {
            TransportEvent::Received(a, b) => (a, b),
            _ => panic!("Received wrong TransportEvent type"),
        };
        // TODO review with net devs
        assert!(node_A.get_uri(recv_id.as_str()).is_some());
        assert_eq!(payload, recv_payload.as_slice());
    }
}

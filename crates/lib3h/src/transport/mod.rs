//! common types and traits for working with Transport instances

pub mod error;
pub mod memory_mock;
pub mod protocol;
pub mod websocket;

#[macro_use]
pub mod transport_test_harness;

pub mod transport_multiplex;
pub use transport_multiplex::TransportMultiplex;

// FIXME
// TODO do we still really need these tests?
//#[cfg(test)]
//pub mod tests {
//    #![allow(non_snake_case)]
//
//    use crate::transport::{protocol::*};
//    use crate::tests::enable_logging_for_test;
//    use url::Url;
//
//    // How many times to call process before asserting if work was done.
//    // Empirically verified to work with just 6- raise this value
//    // if your transport to be tested requires more iterations.
//    const NUM_PROCESS_LOOPS: u8 = 6;
//
//    #[test]
//    fn memory_send_test() {
//        enable_logging_for_test(true);
//        let mut node_A = transport_memory::TransportMemory::new();
//        let mut node_B = transport_memory::TransportMemory::new();
//        let uri_A = Url::parse("mem://a").unwrap();
//        let uri_B = Url::parse("mem://b").unwrap();
//
//        send_test(&mut node_A, &mut node_B, &uri_A, &uri_B);
//    }
//
//
//        #[test]
//        fn wss_send_test() {
//            enable_logging_for_test(true);
//            let mut node_A = TransportWss::with_std_tcp_stream(TlsConfig::Unencrypted);
//            let mut node_B = TransportWss::with_std_tcp_stream(TlsConfig::Unencrypted);
//            let uri_A = Url::parse("wss://127.0.0.1:64529/A").unwrap();
//            let uri_B = Url::parse("wss://127.0.0.1:64530/B").unwrap();
//
//            send_test(&mut node_A, &mut node_B, &uri_A, &uri_B);
//        }
//
//        #[test]
//        fn wss_send_test_tls() {
//            enable_logging_for_test(true);
//            let mut node_A = TransportWss::with_std_tcp_stream(TlsConfig::FakeServer);
//            let mut node_B = TransportWss::with_std_tcp_stream(TlsConfig::FakeServer);
//            let uri_A = Url::parse("wss://127.0.0.1:64531/TLS_A").unwrap();
//            let uri_B = Url::parse("wss://127.0.0.1:64532/TLS_B").unwrap();
//
//            send_test(&mut node_A, &mut node_B, &uri_A, &uri_B);
//        }
//
//    fn send_test(
//        node_A: &mut impl Transport,
//        node_B: &mut impl Transport,
//        uri_A: &Url,
//        uri_B: &Url,
//    ) {
//        // Bind & connect
//        let _bound_uri_a = node_A.bind(uri_A).unwrap();
//        let bound_uri_b = node_B.bind(uri_B).unwrap();
//        let idAB = node_A.connect(&bound_uri_b).unwrap();
//        trace!("bound_uri_b: {}, idAB: {}", bound_uri_b, idAB);
//
//        let (_did_work, _event_list) = node_A.process().unwrap();
//        let (_did_work, _event_list) = node_B.process().unwrap();
//
//        // Send A -> B
//        trace!("Send A -> B");
//        let payload = [1, 2, 3, 4];
//        node_A.send(&[&idAB], &payload).unwrap();
//        let mut did_work = false;
//        let mut event_list = Vec::new();
//
//        // TODO consider making this a while loop with timeout
//        for _x in 0..NUM_PROCESS_LOOPS {
//            let (did_work_B, mut event_list_B) = node_B.process().unwrap();
//            let (_did_work_A, _event_list_A) = node_A.process().unwrap();
//            event_list.append(&mut event_list_B);
//            did_work |= did_work_B;
//        }
//        assert!(did_work);
//        assert!(event_list.len() >= 1);
//        let recv_event = event_list.last().unwrap().clone();
//        let (recv_id, recv_payload) = match recv_event {
//            TransportEvent::ReceivedData(a, b) => (a, b),
//            e => panic!("Received wrong TransportEvent type: {:?}", e),
//        };
//        assert!(node_A.get_uri(idAB.as_str()).is_some());
//        assert!(node_B.get_uri(recv_id.as_str()).is_some());
//
//        debug!(
//            "node_A.get_uri({:?}): {:?}",
//            idAB,
//            node_A.get_uri(idAB.as_str()).unwrap()
//        );
//        debug!(
//            "node_B.get_uri({:?}): {:?}",
//            recv_id,
//            node_B.get_uri(recv_id.as_str()).unwrap()
//        );
//
//        assert_eq!(payload, recv_payload.as_slice());
//        let (_did_work, _event_list) = node_A.process().unwrap();
//
//        // Send B -> A
//        let payload = [4, 2, 1, 3];
//        let id_list = node_B.connection_id_list().unwrap();
//        // referencing node_B's connection list
//        let idBA = id_list[0].clone();
//        node_B.send(&[&idBA], &payload).unwrap();
//        did_work = false;
//        event_list.clear();
//        for _x in 0..NUM_PROCESS_LOOPS {
//            let (did_work_A, mut event_list_A) = node_A.process().unwrap();
//            let (_did_work_B, _event_list_B) = node_B.process().unwrap();
//            did_work |= did_work_A;
//            event_list.append(&mut event_list_A);
//        }
//        assert!(did_work);
//        assert_eq!(event_list.len(), 1);
//        let recv_event = event_list[0].clone();
//        let (recv_id, recv_payload) = match recv_event {
//            TransportEvent::ReceivedData(a, b) => (a, b),
//            _ => panic!("Received wrong TransportEvent type"),
//        };
//        assert!(node_A.get_uri(recv_id.as_str()).is_some());
//        assert!(node_B.get_uri(idBA.as_str()).is_some());
//
//        assert_eq!(payload, recv_payload.as_slice());
//    }
//}

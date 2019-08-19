//! common types and traits for working with Transport instances

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub mod error;
pub mod memory_mock;
pub mod protocol;
pub mod transport_template;
pub mod transport_trait;

use transport_trait::Transport;

/// Hide complexity of Arc<RwLock<dyn Transport>>
/// making it more ergonomic to work with
#[derive(Clone)]
pub struct TransportWrapper<'wrap> {
    inner: Arc<RwLock<dyn Transport + 'wrap>>,
}

impl<'wrap> TransportWrapper<'wrap> {
    /// wrap a concrete Transport into an Arc<RwLock<dyn Transport>>
    pub fn new<T: Transport + 'wrap>(concrete: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(concrete)),
        }
    }

    /// if we already have an Arc<RwLock<dyn Transport>>,
    /// us it directly as our inner
    pub fn assume(inner: Arc<RwLock<dyn Transport + 'wrap>>) -> Self {
        Self { inner }
    }

    /// get an immutable ref to Transport trait object
    pub fn as_ref(&self) -> RwLockReadGuard<'_, dyn Transport + 'wrap> {
        self.inner.read().expect("failed to obtain read lock")
    }

    /// get a mutable ref to Transport trait object
    pub fn as_mut(&self) -> RwLockWriteGuard<'_, dyn Transport + 'wrap> {
        self.inner.write().expect("failed to obtain write lock")
    }
}

#[cfg(test)]
pub mod tests {
    #![allow(non_snake_case)]

    use crate::{
        transport::{
            memory_mock::transport_memory, protocol::TransportEvent, transport_trait::Transport,
        },
        transport_wss::{TlsConfig, TransportWss},
    };

    use crate::tests::enable_logging_for_test;
    use url::Url;

    // How many times to call process before asserting if work was done.
    // Empirically verified to work with just 6- raise this value
    // if your transport to be tested requires more iterations.
    const NUM_PROCESS_LOOPS: u8 = 6;

    #[test]
    fn memory_send_test() {
        enable_logging_for_test(true);
        let mut node_A = transport_memory::TransportMemory::new();
        let mut node_B = transport_memory::TransportMemory::new();
        let uri_A = Url::parse("mem://a").unwrap();
        let uri_B = Url::parse("mem://b").unwrap();

        send_test(&mut node_A, &mut node_B, &uri_A, &uri_B);
    }

    #[test]
    fn wss_send_test_un() {
        enable_logging_for_test(true);
        let mut node_A = TransportWss::with_std_tcp_stream(TlsConfig::Unencrypted);
        let mut node_B = TransportWss::with_std_tcp_stream(TlsConfig::Unencrypted);
        let uri_A = Url::parse("wss://127.0.0.1:64529/A").unwrap();
        let uri_B = Url::parse("wss://127.0.0.1:64530/B").unwrap();

        send_test(&mut node_A, &mut node_B, &uri_A, &uri_B);
    }

    #[test]
    fn wss_send_test_tls() {
        enable_logging_for_test(true);
        let mut node_A = TransportWss::with_std_tcp_stream(TlsConfig::FakeServer);
        let mut node_B = TransportWss::with_std_tcp_stream(TlsConfig::FakeServer);
        let uri_A = Url::parse("wss://127.0.0.1:64531/TLS_A").unwrap();
        let uri_B = Url::parse("wss://127.0.0.1:64532/TLS_B").unwrap();

        send_test(&mut node_A, &mut node_B, &uri_A, &uri_B);
    }

    fn proc(node: &mut impl Transport) -> (bool, Vec<TransportEvent>) {
        std::thread::sleep(std::time::Duration::from_millis(5));
        let out = node.process().unwrap();
        debug!("process node {:?}", &out);
        out
    }

    fn send_test(
        node_A: &mut impl Transport,
        node_B: &mut impl Transport,
        uri_A: &Url,
        uri_B: &Url,
    ) {
        // Connect
        let actual_bind_uri_a = node_A.bind_sync(uri_A.clone()).unwrap();
        let actual_bind_uri_b = node_B.bind_sync(uri_B.clone()).unwrap();
        debug!("actual_bind_uri_a: {}", actual_bind_uri_a);
        debug!("actual_bind_uri_b: {}", actual_bind_uri_b);
        node_A
            .connect("".to_string(), actual_bind_uri_b.clone())
            .unwrap();
        node_B
            .connect("".to_string(), actual_bind_uri_a.clone())
            .unwrap();

        proc(node_A);
        proc(node_B);

        // Send A -> B
        let payload = [1, 2, 3, 4];
        node_A
            .send("".to_string(), actual_bind_uri_b.clone(), payload.to_vec())
            .unwrap();
        let mut did_work = false;
        let mut event_list = Vec::new();

        // TODO consider making this a while loop with timeout
        for _x in 0..NUM_PROCESS_LOOPS {
            let (did_work_B, mut event_list_B) = proc(node_B);
            let (_did_work_A, _event_list_A) = proc(node_A);
            event_list.append(&mut event_list_B);
            did_work |= did_work_B;
        }
        assert!(did_work);
        assert!(event_list.len() >= 1);
        let recv_event = event_list.last().unwrap().clone();
        let (_, recv_payload) = match recv_event {
            TransportEvent::ReceivedData { address, payload } => (address, payload),
            e => panic!("Received wrong TransportEvent type: {:?}", e),
        };

        assert_eq!(payload, recv_payload.as_slice());

        // Send B -> A
        let payload = [4, 2, 1, 3];
        // referencing node_B's connection list
        node_B
            .send("".to_string(), actual_bind_uri_a.clone(), payload.to_vec())
            .unwrap();
        did_work = false;
        event_list.clear();
        for _x in 0..NUM_PROCESS_LOOPS {
            let (did_work_A, mut event_list_A) = proc(node_A);
            let (_did_work_B, _event_list_B) = proc(node_B);
            did_work |= did_work_A;
            event_list.append(&mut event_list_A);
        }
        assert!(did_work);
        assert_eq!(event_list.len(), 1);
        let recv_event = event_list[0].clone();
        let (_, recv_payload) = match recv_event {
            TransportEvent::ReceivedData { address, payload } => (address, payload),
            _ => panic!("Received wrong TransportEvent type"),
        };

        assert_eq!(payload, recv_payload.as_slice());
    }
}

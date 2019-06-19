use crate::transport::{
    error::TransportResult,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    TransportId, TransportIdRef,
};
use lib3h_protocol::DidWork;

/// Wraps any transport and adds cryptography
pub struct TransportCrypto<T: Transport> {
    inner_transport: T,
}

/// Constructor
/// FIXME: Consume inner_tranport or have it be a reference?
impl<T: Transport> TransportCrypto<T> {
    pub fn new(inner_transport: T) -> Self {
        TransportCrypto { inner_transport }
    }
}

/// Implement Transport trait by composing inner transport
/// FIXME passthrough for now
impl<T: Transport> Transport for TransportCrypto<T> {
    fn connect(&mut self, uri: &str) -> TransportResult<TransportId> {
        self.inner_transport.connect(&uri)
    }
    fn close(&mut self, id: &TransportIdRef) -> TransportResult<()> {
        self.inner_transport.close(id)
    }

    fn close_all(&mut self) -> TransportResult<()> {
        self.inner_transport.close_all()
    }

    fn send(&mut self, id_list: &[&TransportIdRef], payload: &[u8]) -> TransportResult<()> {
        self.inner_transport.send(id_list, payload)
    }

    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        self.inner_transport.send_all(payload)
    }

    fn bind(&mut self, url: &str) -> TransportResult<String> {
        self.inner_transport.bind(url)
    }

    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.inner_transport.post(command)
    }

    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        self.inner_transport.process()
    }

    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        self.inner_transport.transport_id_list()
    }

    fn get_uri(&self, id: &TransportIdRef) -> Option<String> {
        self.inner_transport.get_uri(id)
    }
}

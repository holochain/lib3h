use crate::transport::{
    error::TransportResult,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    ConnectionId, ConnectionIdRef,
};
use lib3h_protocol::DidWork;

use url::Url;

/// Wraps any transport and adds cryptography
pub struct TransportCrypto {
    inner_transport: Box<dyn Transport>,
}

/// Constructor
/// TODO #177 - Consume inner_tranport or have it be a reference?
impl TransportCrypto {
    pub fn new(inner_transport: Box<dyn Transport>) -> Self {
        TransportCrypto { inner_transport }
    }
}

/// Implement Transport trait by composing inner transport
/// TODO #177 - passthrough for now
impl Transport for TransportCrypto {
    fn connect(&mut self, uri: &Url) -> TransportResult<ConnectionId> {
        self.inner_transport.connect(&uri)
    }
    fn close(&mut self, id: &ConnectionIdRef) -> TransportResult<()> {
        self.inner_transport.close(id)
    }

    fn close_all(&mut self) -> TransportResult<()> {
        self.inner_transport.close_all()
    }

    fn send(&mut self, id_list: &[&ConnectionIdRef], payload: &[u8]) -> TransportResult<()> {
        self.inner_transport.send(id_list, payload)
    }

    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        self.inner_transport.send_all(payload)
    }

    fn bind(&mut self, url: &Url) -> TransportResult<Url> {
        self.inner_transport.bind(url)
    }

    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.inner_transport.post(command)
    }

    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        self.inner_transport.process()
    }

    fn connection_id_list(&self) -> TransportResult<Vec<ConnectionId>> {
        self.inner_transport.connection_id_list()
    }

    fn get_uri(&self, id: &ConnectionIdRef) -> Option<Url> {
        self.inner_transport.get_uri(id)
    }
}

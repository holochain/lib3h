use crate::transport::{
    error::TransportResult,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    ConnectionId, ConnectionIdRef, TransportWrapper,
};
use lib3h_protocol::DidWork;

use url::Url;

/// Wraps any transport and adds cryptography
pub struct TransportCrypto<'crypto> {
    inner_transport: TransportWrapper<'crypto>,
}

/// Constructor
/// TODO #177 - Consume inner_tranport or have it be a reference?
impl<'crypto> TransportCrypto<'crypto> {
    pub fn new(inner_transport: TransportWrapper<'crypto>) -> Self {
        TransportCrypto { inner_transport }
    }
}

/// Implement Transport trait by composing inner transport
/// TODO #177 - passthrough for now
impl<'crypto> Transport for TransportCrypto<'crypto> {
    fn connect(&mut self, uri: &Url) -> TransportResult<ConnectionId> {
        self.inner_transport.as_mut().connect(&uri)
    }
    fn close(&mut self, id: &ConnectionIdRef) -> TransportResult<()> {
        self.inner_transport.as_mut().close(id)
    }

    fn close_all(&mut self) -> TransportResult<()> {
        self.inner_transport.as_mut().close_all()
    }

    fn send(&mut self, id_list: &[&ConnectionIdRef], payload: &[u8]) -> TransportResult<()> {
        self.inner_transport.as_mut().send(id_list, payload)
    }

    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        self.inner_transport.as_mut().send_all(payload)
    }

    fn bind(&mut self, url: &Url) -> TransportResult<Url> {
        self.inner_transport.as_mut().bind(url)
    }

    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.inner_transport.as_mut().post(command)
    }

    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        self.inner_transport.as_mut().process()
    }

    fn connection_id_list(&self) -> TransportResult<Vec<ConnectionId>> {
        self.inner_transport.as_ref().connection_id_list()
    }

    fn get_uri(&self, id: &ConnectionIdRef) -> Option<Url> {
        self.inner_transport.as_ref().get_uri(id)
    }
}

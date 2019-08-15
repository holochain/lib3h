use crate::transport::{error::TransportResult, protocol::*};

use url::Url;

use lib3h_protocol::DidWork;

/// Represents a pool of connections to remote nodes.
/// Methods are for synchronous processing.
/// Otherwise use `post()` & `process()` for aysnchronous processing.
pub trait Transport {
    /// Send a command for later processing
    fn post(&mut self, command: TransportCommand) -> TransportResult<()>;

    /// Process TransportProtocol messages received from owner and
    /// also poll TransportEvents received from the network.
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)>;

    /// get a list of all open connection addresses
    fn connection_list(&self) -> TransportResult<Vec<Url>>;

    /// THIS IS A HACK, REMOVE IT
    fn bind_sync(&mut self, spec: Url) -> TransportResult<Url>;

    // -- Asynchronous -- //
    // the following are just helpers to post(TransportCommand::*)

    /// Bind to a network interface
    fn bind(&mut self, request_id: RequestId, spec: Url) -> TransportResult<()> {
        self.post(TransportCommand::Bind { request_id, spec })
    }

    /// establish a connection to a remote node
    fn connect(&mut self, request_id: RequestId, address: Url) -> TransportResult<()> {
        self.post(TransportCommand::Connect {
            request_id,
            address,
        })
    }

    /// send a payload to remote nodes
    fn send(
        &mut self,
        request_id: RequestId,
        address: Url,
        payload: Vec<u8>,
    ) -> TransportResult<()> {
        self.post(TransportCommand::SendMessage {
            request_id,
            address,
            payload,
        })
    }
}

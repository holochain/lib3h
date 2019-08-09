use crate::transport::{
    error::TransportResult,
    protocol::{TransportCommand, TransportEvent},
    ConnectionId, ConnectionIdRef,
};

use url::Url;

use lib3h_protocol::DidWork;

/// Represents a pool of connections to remote nodes.
/// Methods are for synchronous processing.
/// Otherwise use `post()` & `process()` for aysnchronous processing.
pub trait Transport {
    // -- Synchronous -- //
    /// establish a connection to a remote node
    fn connect(&mut self, uri: &Url) -> TransportResult<ConnectionId>;
    /// close an existing open connection
    fn close(&mut self, id: &ConnectionIdRef) -> TransportResult<()>;
    /// close all existing open connections
    fn close_all(&mut self) -> TransportResult<()>;
    /// send a payload to remote nodes
    /// note: you probably mean to `post` a TransportCommand::SendReliable
    /// this function does that internally... but you can't get the response
    //fn send(&mut self, id_list: &[&ConnectionIdRef], payload: &[u8]) -> TransportResult<()>;
    /// send a payload to all remote nodes
    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()>;
    /// Bind to a network interface
    /// Return the advertise
    fn bind(&mut self, url: &url::Url) -> TransportResult<Url>;

    // -- Asynchronous -- //
    /// Send a command for later processing
    fn post(&mut self, command: TransportCommand) -> TransportResult<()>;
    /// Process TransportProtocol messages received from owner and
    /// also poll TransportEvents received from the network.
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)>;

    // -- Getters -- //
    /// get a list of all open transport ids
    fn connection_id_list(&self) -> TransportResult<Vec<ConnectionId>>;
    /// get uri from a connectionId
    fn get_uri(&self, id: &ConnectionIdRef) -> Option<Url>;
}

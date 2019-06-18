use crate::transport::{
    error::TransportResult,
    protocol::{TransportCommand, TransportEvent},
    TransportId, TransportIdRef,
};

use lib3h_protocol::DidWork;

/// Represents a pool of connections to remote nodes.
/// Methods are for synchronous processing.
/// Otherwise use `post()` & `process()` for aysnchronous processing.
pub trait Transport {
    // -- Synchronous -- //
    /// establish a connection to a remote node
    fn connect(&mut self, uri: &str) -> TransportResult<TransportId>;
    /// close an existing open connection
    fn close(&mut self, id: &TransportIdRef) -> TransportResult<()>;
    /// close all existing open connections
    fn close_all(&mut self) -> TransportResult<()>;
    /// send a payload to remote nodes
    fn send(&mut self, id_list: &[&TransportIdRef], payload: &[u8]) -> TransportResult<()>;
    /// send a payload to all remote nodes
    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()>;
    /// Bind to a network interface
    /// Return the advertise
    fn bind(&mut self, url: &str) -> TransportResult<String>;

    // -- Asynchronous -- //
    /// Send a command for later processing
    fn post(&mut self, command: TransportCommand) -> TransportResult<()>;
    /// Process TransportProtocol messages received from owner and
    /// also poll TransportEvents received from the network.
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)>;

    // -- Getters -- //
    /// get a list of all open transport ids
    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>>;
}

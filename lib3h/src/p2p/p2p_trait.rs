use crate::p2p::p2p_event::P2pEvent;
use holochain_lib3h_protocol::{DidWork, Lib3hResult};

/// Composition of a Connection, Dht, RuntimeStore, PeerStore, and DataStore
/// to provide a higher-level Connection interface.
pub trait P2p {
    /// Lifecycle
    fn connect(&mut self, url: String) -> Lib3hResult<()>;
    fn close(&self, peer_address: String) -> Lib3hResult<()>;
    /// Comms
    fn publish_reliable(&self, peer_list: Vec<String>, data: Vec<u8>) -> Lib3hResult<()>;
    fn publish_unreliable(&self, peer_list: Vec<String>, data: Vec<u8>) -> Lib3hResult<()>;
    fn request_reliable(&self, peer_list: Vec<String>, data: Vec<u8>) -> Lib3hResult<()>;
    fn respond_reliable(&self, msg_id: String, data: Vec<u8>) -> Lib3hResult<()>;
    /// Processing
    fn post(&mut self, evt: P2pEvent) -> Lib3hResult<()>;
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<P2pEvent>)>;
    /// Getters
    fn id(&self) -> String;
    fn advertise(&self) -> String;
}

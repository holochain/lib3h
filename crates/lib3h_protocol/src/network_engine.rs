use crate::{
    protocol_client::Lib3hClientProtocol, protocol_server::Lib3hServerProtocol, DidWork,
    Lib3hResult,
};

use url::Url;

/// Common interface for all types of network modules to be used by the Lib3hWorker
pub trait NetworkEngine {
    /// Post a protocol message from core -> lib3h
    fn post(&mut self, data: Lib3hClientProtocol) -> Lib3hResult<()>;
    /// A single iteration of the networking process loop
    /// (should be called frequently to not cpu starve networking)
    /// Returns a vector of protocol messages core is required to handle.
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)>;
    /// Get qualified transport address
    fn advertise(&self) -> Url;
}

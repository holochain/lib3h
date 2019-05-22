use crate::{protocol::Lib3hProtocol, DidWork, Lib3hResult};

/// Common interface for all types of network modules to be used by the Lib3hWorker
pub trait NetworkEngine {
    /// Start network communications
    fn run(&self) -> Lib3hResult<()>;
    /// Stop network communications
    fn stop(&self) -> Lib3hResult<()>;
    /// Terminate module. Perform some cleanup.
    fn terminate(&self) -> Lib3hResult<()>;
    /// Post a protocol message from core -> lib3h
    fn post(&mut self, data: Lib3hProtocol) -> Lib3hResult<()>;
    /// A single iteration of the networking process loop
    /// (should be called frequently to not cpu starve networking)
    /// Returns a vector of protocol messages core is required to handle.
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<Lib3hProtocol>)>;
    /// Get qualified transport address
    fn advertise(&self) -> String;
}

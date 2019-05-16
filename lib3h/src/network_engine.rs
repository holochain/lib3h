use holochain_lib3h_protocol::{protocol::Lib3hProtocol, Lib3hResult};
/// Common interface for all types of network modules to be used by the Lib3hWorker
pub trait NetworkEngine {
    /// Start network communications
    fn run(&self) -> Lib3hResult<()>;
    /// Stop network communications
    fn stop(&self) -> Lib3hResult<()>;
    /// Terminate module. Perform some cleanup.
    fn terminate(&self) -> Lib3hResult<()>;
    /// Handle some data message sent by the local Client
    fn serve(&self, data: Lib3hProtocol) -> Lib3hResult<()>;
    /// Get qualified transport address
    fn advertise(&self) -> String;
}

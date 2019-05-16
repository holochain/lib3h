
/// Common interface for all types of network modules to be used by the Lib3hWorker
pub trait NetworkEngine {
    /// Start network communications
    fn run(&mut self) -> NetResult<()>;
    /// Stop network communications
    fn stop(&mut self) -> NetResult<()>;
    /// Terminate module. Perform some cleanup.
    fn terminate(&self) -> NetResult<()>;
    /// Handle some data message sent by the local Client
    fn serve(&mut self, data: Protocol) -> NetResult<()>;
    /// Get qualified transport address
    fn advertise(&self) -> String;
}

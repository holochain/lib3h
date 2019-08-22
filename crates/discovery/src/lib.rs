//! Discovery trait definition.

pub mod error;
pub use error::DiscoveryResult;

pub trait Discovery {
    fn advertise(&mut self) -> DiscoveryResult<()>;
    fn discover(&mut self) -> DiscoveryResult<()>;
    fn release(&mut self) -> DiscoveryResult<()>;
    fn flush(&mut self) -> DiscoveryResult<()>;
}

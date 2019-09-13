//! Discovery trait definition.

pub mod error;
pub use error::DiscoveryResult;
use url::Url;

pub trait Discovery {
    fn advertise(&mut self) -> DiscoveryResult<()>;
    fn discover(&mut self) -> DiscoveryResult<Vec<Url>>;
    fn release(&mut self) -> DiscoveryResult<()>;
    fn flush(&mut self) -> DiscoveryResult<()>;
}

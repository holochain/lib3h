//! Discovery trait definition.

extern crate lib3h_protocol;

pub mod error;
pub use error::DiscoveryResult;
use lib3h_protocol::uri::Lib3hUri;

pub trait Discovery {
    fn advertise(&mut self) -> DiscoveryResult<()>;
    fn discover(&mut self) -> DiscoveryResult<Vec<Lib3hUri>>;
    fn release(&mut self) -> DiscoveryResult<()>;
    fn flush(&mut self) -> DiscoveryResult<()>;
}

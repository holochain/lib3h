//! Discovery trait definition.

pub mod error;

use crate::{discovery::error::DiscoveryResult, uri::Lib3hUri};

pub trait Discovery {
    fn advertise(&mut self) -> DiscoveryResult<()>;
    fn discover(&mut self) -> DiscoveryResult<Vec<Lib3hUri>>;
    fn release(&mut self) -> DiscoveryResult<()>;
    fn flush(&mut self) -> DiscoveryResult<()>;
}

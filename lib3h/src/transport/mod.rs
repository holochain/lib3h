//! common types and traits for working with Transport instances

pub mod error;
pub mod protocol;
pub mod transport_trait;

/// a connection identifier
pub type TransportId = String;
pub type TransportIdRef = str;

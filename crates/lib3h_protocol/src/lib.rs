//! This module provides the api definition for working with lib3h

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate rmp_serde as rmps;

pub mod data_types;
pub mod network_engine;
pub mod protocol_client;
pub mod protocol_server;

/// Opaque Address Bytes
pub type Address = Vec<u8>;
pub type AddressRef = [u8];

/// type name for a bool indicating if work was done during a `process()`
pub type DidWork = bool;

/// TODO: To replace with our own custom Error handling
use failure::Error;
pub type Lib3hResult<T> = Result<T, Error>;

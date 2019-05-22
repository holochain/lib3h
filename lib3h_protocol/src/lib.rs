//! This module provides the api definition for working with lib3h

pub mod data_types;
pub mod network_engine;
pub mod protocol;

/// Opaque Address Bytes
pub type Address = Vec<u8>;

/// type name for a bool indicating if work was done during a `process()`
pub type DidWork = bool;

/// TODO: To replace with our own custom Error handling
use failure::Error;
pub type Lib3hResult<T> = Result<T, Error>;

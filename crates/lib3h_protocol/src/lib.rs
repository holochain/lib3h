//! This module provides the api definition for working with lib3h

extern crate backtrace;
extern crate holochain_persistence_api;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate rmp_serde;

pub mod data_types;
pub mod network_engine;
pub mod protocol_client;
pub mod protocol_server;

/// string encoded address type
pub type Address = holochain_persistence_api::hash::HashString;

/// type name for a bool indicating if work was done during a `process()`
pub type DidWork = bool;

pub mod error;

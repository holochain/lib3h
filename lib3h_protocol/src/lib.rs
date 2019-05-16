#![feature(try_from)]

//! This module provides the api definition for working with lib3h

#[macro_use]
extern crate serde_derive;

pub mod data_types;
pub mod protocol;

/// Opaque Address Bytes
pub type Address = Vec<u8>;


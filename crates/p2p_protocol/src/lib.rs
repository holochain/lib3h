//! Lib3h Protocol definition for inter-node p2p communication.

extern crate capnp;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

#[allow(dead_code)]
#[allow(clippy::all)]
#[rustfmt::skip]
mod multiplex_capnp;
#[allow(dead_code)]
#[allow(clippy::all)]
#[rustfmt::skip]
mod p2p_capnp;
#[allow(dead_code)]
#[allow(clippy::all)]
#[rustfmt::skip]
mod transit_encoding_capnp;

pub mod p2p;

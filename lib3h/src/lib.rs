// error_chain recursion limit
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
extern crate libsodacon;
extern crate rmp_serde;
extern crate serde;
#[macro_use]
extern crate serde_derive;

pub mod errors;
pub mod message;
pub mod node;

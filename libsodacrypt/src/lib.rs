/*!
This is currently a thin abstraction wrapper around libsodium/sodiumoxide.

Sacrifices some minor memory efficiencies to provide a more straight-forward interoperative api.
*/

// error_chain recursion limit
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
extern crate sodiumoxide;

pub mod errors;
pub mod hash;
pub mod init;
pub mod kx;
pub mod rand;
pub mod sign;
pub mod sym;

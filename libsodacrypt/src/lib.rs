/*!
This is currently a thin abstraction wrapper around libsodium/sodiumoxide.

Sacrifices some minor memory efficiencies to provide a more straight-forward interoperative api.
*/

extern crate sodiumoxide;

pub mod error;
pub mod hash;
pub mod kx;
pub mod rand;
pub mod sign;
pub mod sym;

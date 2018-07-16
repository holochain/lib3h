// error_chain recursion limit
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
extern crate hex;
extern crate inflector;
#[macro_use]
extern crate lazy_static;
extern crate libsodacrypt;
extern crate regex;
extern crate rmp_serde;
extern crate serde;
#[macro_use]
extern crate serde_derive;

pub mod errors;
pub mod net;
pub mod node;

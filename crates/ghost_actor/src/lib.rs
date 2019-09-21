#![recursion_limit = "128"]
extern crate crossbeam_channel;
extern crate inflector;
//#[macro_use]
extern crate lazy_static;
extern crate proc_macro2;
//#[macro_use]
extern crate syn;
#[allow(unused_imports)]
#[macro_use]
extern crate quote;

mod ghost_error;
pub use ghost_error::{ErrorKind, GhostError, GhostResult};

pub mod code_gen;

pub mod prelude {

    pub use super::*;
}

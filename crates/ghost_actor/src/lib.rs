#![recursion_limit = "128"]
extern crate crossbeam_channel;
extern crate inflector;
//#[macro_use]
extern crate lazy_static;
extern crate proc_macro2;
#[macro_use]
extern crate shrinkwraprs;
//#[macro_use]
extern crate syn;
#[allow(unused_imports)]
#[macro_use]
extern crate quote;

#[derive(Shrinkwrap, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[shrinkwrap(mutable)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn new() -> Self {
        Self::with_prefix("")
    }

    pub fn with_prefix(prefix: &str) -> Self {
        Self(format!("{}{}", prefix, nanoid::simple()))
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        RequestId(s)
    }
}

impl From<RequestId> for String {
    fn from(r: RequestId) -> Self {
        r.0
    }
}

mod ghost_error;
pub use ghost_error::*;

pub type GhostHandlerCb<'lt, T> = Box<dyn FnOnce(T) -> GhostResult<()> + 'lt + Send + Sync>;

pub type GhostResponseCb<'lt, X, T> =
    Box<dyn FnOnce(&mut X, GhostResult<T>) -> GhostResult<()> + 'lt + Send + Sync>;

mod ghost_protocol;
pub use ghost_protocol::*;

#[allow(dead_code)]
#[cfg(test)]
mod test_proto {
    include!(concat!(env!("OUT_DIR"), "/test_proto.rs"));
}

mod ghost_system;
pub use ghost_system::*;

mod ghost_channel;
pub use ghost_channel::*;

mod ghost_actor;
//pub use crate::ghost_actor::*;

pub mod code_gen;

pub mod prelude {

    pub use super::*;
}

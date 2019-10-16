#![recursion_limit = "128"]
extern crate crossbeam_channel;
extern crate holochain_tracing;
extern crate inflector;
//#[macro_use]
extern crate lazy_static;
extern crate lib3h_zombie_actor;
extern crate lock_api;
extern crate parking_lot;
extern crate proc_macro2;
#[macro_use]
extern crate shrinkwraprs;
//#[macro_use]
extern crate syn;
#[allow(unused_imports)]
#[macro_use]
extern crate quote;

pub use lib3h_zombie_actor::{ErrorKind as GhostErrorKind, GhostError, GhostResult};

mod ghost_mutex;
pub use ghost_mutex::*;

mod ghost_deep_ref;
pub(crate) use ghost_deep_ref::*;

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

mod ghost_protocol;
pub use ghost_protocol::*;

mod ghost_tracker;
pub use ghost_tracker::*;

mod ghost_actor;
pub use crate::ghost_actor::*;

mod ghost_system;
pub use ghost_system::*;

pub mod code_gen;

pub mod prelude {
    pub use super::*;
}

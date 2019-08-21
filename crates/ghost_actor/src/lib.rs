#[macro_use]
extern crate shrinkwraprs;

#[derive(Shrinkwrap, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[shrinkwrap(mutable)]
pub struct DidWork(pub bool);

impl From<bool> for DidWork {
    fn from(b: bool) -> Self {
        DidWork(b)
    }
}

impl From<DidWork> for bool {
    fn from(d: DidWork) -> Self {
        d.0
    }
}

#[derive(Shrinkwrap, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[shrinkwrap(mutable)]
pub struct RequestId(pub String);

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

mod ghost_tracker;
pub use ghost_tracker::GhostTracker;

mod ghost_actor;
pub use ghost_actor::GhostActor;

pub mod prelude {
    pub use super::{GhostActor, GhostTracker};
}

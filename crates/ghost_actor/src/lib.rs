extern crate nanoid;
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

mod ghost_tracker;
pub use ghost_tracker::{GhostCallback, GhostCallbackData, GhostTracker};

mod ghost_actor_state;
pub use ghost_actor_state::GhostActorState;

mod ghost_actor;
pub use ghost_actor::GhostActor;

pub mod prelude {
    pub use super::{GhostActor, GhostActorState, GhostCallback, GhostCallbackData, GhostTracker};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;

    #[allow(dead_code)]
    mod transport_protocol {
        #[derive(Debug)]
        pub enum RequestFromParent {
            Bind { url: String },
        }

        #[derive(Debug)]
        pub enum ResponseToParent {
            BindResult { bound_url: Result<String, String> },
        }

        #[derive(Debug)]
        pub enum RequestAsChild {
            IncomingConnection { address: String },
        }

        #[derive(Debug)]
        pub enum ResponseAsChild {
            Idle,
        }
    }

    use transport_protocol::*;

    struct WssTransport {
        actor_state:
            Option<GhostActorState<RequestAsChild, ResponseAsChild, ResponseToParent, String>>,
    }

    impl WssTransport {
        pub fn new() -> Self {
            Self {
                actor_state: Some(GhostActorState::new()),
            }
        }
    }

    impl GhostActor<RequestAsChild, ResponseAsChild, RequestFromParent, ResponseToParent, String>
        for WssTransport
    {
        fn as_any(&mut self) -> &mut dyn Any {
            &mut *self
        }

        fn get_actor_state(
            &mut self,
        ) -> &mut GhostActorState<RequestAsChild, ResponseAsChild, ResponseToParent, String>
        {
            self.actor_state.as_mut().unwrap()
        }

        fn take_actor_state(
            &mut self,
        ) -> GhostActorState<RequestAsChild, ResponseAsChild, ResponseToParent, String> {
            std::mem::replace(&mut self.actor_state, None).unwrap()
        }

        fn put_actor_state(
            &mut self,
            actor_state: GhostActorState<RequestAsChild, ResponseAsChild, ResponseToParent, String>,
        ) {
            std::mem::replace(&mut self.actor_state, Some(actor_state));
        }

        // our parent is making a request of us
        fn request(&mut self, request_id: Option<RequestId>, request: RequestFromParent) {
            match request {
                RequestFromParent::Bind { url: _u } => {
                    // do some internal bind
                    // we get a bound_url
                    let bound_url = "bound_url".to_string();
                    // respond to our parent
                    if let Some(request_id) = request_id {
                        self.get_actor_state().respond_to_parent(
                            request_id,
                            ResponseToParent::BindResult {
                                bound_url: Ok(bound_url),
                            },
                        );
                    }
                }
            }
        }

        fn process_concrete(&mut self) -> Result<DidWork, String> {
            self.get_actor_state().send_request_to_parent(
                std::time::Duration::from_millis(2000),
                RequestAsChild::IncomingConnection {
                    address: "test".to_string(),
                },
                Box::new(|_m, r| {
                    println!("got: {:?}", r);
                    Ok(())
                }),
            );
            Ok(true.into())
        }
    }

    type TransportActor = dyn GhostActor<
        RequestAsChild,
        ResponseAsChild,
        RequestFromParent,
        ResponseToParent,
        String,
    >;

    #[test]
    fn test_wss_transport() {
        let mut t_actor: Box<TransportActor> = Box::new(WssTransport::new());
        t_actor.process().unwrap();
        for (rid, ev) in t_actor.drain_requests() {
            println!("got: {:?} {:?}", rid, ev);
            if let Some(rid) = rid {
                t_actor.respond(rid, ResponseAsChild::Idle).unwrap();
            }
        }
    }
}

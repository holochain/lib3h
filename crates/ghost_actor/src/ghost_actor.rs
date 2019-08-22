use crate::{DidWork, GhostActorState, RequestId};
use std::any::Any;

pub trait GhostActor<RequestAsChild, ResponseAsChild, RequestFromParent, ResponseToParent, E> {
    fn as_any(&mut self) -> &mut dyn Any;

    fn get_actor_state(
        &mut self,
    ) -> &mut GhostActorState<RequestAsChild, ResponseAsChild, ResponseToParent, E>;

    fn take_actor_state(
        &mut self,
    ) -> GhostActorState<RequestAsChild, ResponseAsChild, ResponseToParent, E>;

    fn put_actor_state(
        &mut self,
        actor_state: GhostActorState<RequestAsChild, ResponseAsChild, ResponseToParent, E>,
    );

    fn process(&mut self) -> Result<DidWork, E> {
        let mut actor_state = self.take_actor_state();
        actor_state.process(self.as_any())?;
        self.put_actor_state(actor_state);
        self.process_concrete()
    }

    fn process_concrete(&mut self) -> Result<DidWork, E>;

    // our parent is making a request of us
    fn request(&mut self, request_id: Option<RequestId>, request: RequestFromParent);

    // these are response to our parent from the request they made of us
    fn drain_responses(&mut self) -> Vec<(RequestId, ResponseToParent)> {
        self.get_actor_state().drain_responses()
    }

    // called by parent, these are our requests goint to them
    fn drain_requests(&mut self) -> Vec<(Option<RequestId>, RequestAsChild)> {
        self.get_actor_state().drain_requests()
    }

    // called by parent, these are responses to requests in drain_request
    fn respond(&mut self, request_id: RequestId, response: ResponseAsChild) -> Result<(), E> {
        let mut actor_state = self.take_actor_state();
        let out = actor_state.handle_response(self.as_any(), request_id, response);
        self.put_actor_state(actor_state);
        out
    }
}

use crate::{DidWork, GhostActorState, RequestId};

pub trait GhostActor<
    'ga,
    GA,
    RequestAsChild,
    ResponseAsChild,
    RequestFromParent,
    ResponseToParent,
    E,
>
{
    fn as_mut(&mut self) -> &mut GA;

    fn get_actor_state(
        &mut self,
    ) -> &mut GhostActorState<'ga, GA, RequestAsChild, ResponseAsChild, ResponseToParent, E>;

    fn take_actor_state(
        &mut self,
    ) -> GhostActorState<'ga, GA, RequestAsChild, ResponseAsChild, ResponseToParent, E>;

    fn put_actor_state(
        &mut self,
        actor_state: GhostActorState<'ga, GA, RequestAsChild, ResponseAsChild, ResponseToParent, E>,
    );

    fn process(&mut self) -> Result<DidWork, E> {
        let mut actor_state = self.take_actor_state();
        actor_state.process(self.as_mut())?;
        self.put_actor_state(actor_state);
        Ok(true.into())
    }

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

    // called by parest, these are responses to requests in drain_request
    fn respond(&mut self, request_id: RequestId, response: ResponseAsChild) -> Result<(), E> {
        let mut actor_state = self.take_actor_state();
        let out = actor_state.handle_response(self.as_mut(), request_id, response);
        self.put_actor_state(actor_state);
        out
    }
}

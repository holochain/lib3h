use crate::{DidWork, GhostTracker, RequestId};

pub trait GhostActor<'ga, GA, FromChild, ToChild, FromParent, ToParent, E> {
    fn as_mut(&mut self) -> &mut GA;

    fn get_tracker(&mut self) -> &mut GhostTracker<'ga, GA, FromChild, ToChild, ToParent, E>;

    fn take_tracker(&mut self) -> GhostTracker<'ga, GA, FromChild, ToChild, ToParent, E>;

    fn put_tracker(&mut self, tracker: GhostTracker<'ga, GA, FromChild, ToChild, ToParent, E>);

    fn process(&mut self) -> Result<DidWork, E> {
        self.get_tracker().process()
    }

    // our parent is making a request DOWN to us
    fn request(&mut self, request_id: Option<RequestId>, request: FromParent);

    // these are response to our parent from the request they made DOWN to us
    fn drain_responses(&mut self) -> Vec<(RequestId, ToParent)> {
        self.get_tracker().drain_responses()
    }

    // called by parent, these are our requests going UP
    fn drain_requests(&mut self) -> Vec<(Option<RequestId>, FromChild)> {
        self.get_tracker().drain_requests()
    }

    // called by parest, these are responses to requests in drain_request
    fn respond(&mut self, request_id: RequestId, response: ToChild) {
        let mut tracker = self.take_tracker();
        tracker.handle_out_response(self.as_mut(), request_id, response);
        self.put_tracker(core);
    }
}

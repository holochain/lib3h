use crate::{WorkWasDone, GhostChannel, RequestId};
use std::any::Any;

pub trait GhostActor<
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    E,
>
{
    fn as_any(&mut self) -> &mut dyn Any;

    fn channel(&mut self) -> &mut GhostChannel<RequestToParent, RequestToParentResponse, RequestToChild, RequestToChildResponse, E>;

    fn process(&mut self) -> Result<WorkWasDone, E> {
        self.channel().process();
        /*
        let mut actor_state = self.take_actor_state();
        actor_state.process(self.as_any())?;
        self.put_actor_state(actor_state);
        */
        self.process_concrete()
    }

    fn process_concrete(&mut self) -> Result<WorkWasDone, E>;
}

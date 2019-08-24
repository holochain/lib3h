use crate::{GhostChannel, GhostResult, WorkWasDone};
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
    fn take_parent_channel(
        &mut self,
    ) -> Option<
        GhostChannel<
            RequestToChild,
            RequestToChildResponse,
            RequestToParent,
            RequestToParentResponse,
            E,
        >,
    >;
    fn process(&mut self) -> GhostResult<WorkWasDone> {
        self.process_concrete()
    }
    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        Ok(false.into())
    }
}

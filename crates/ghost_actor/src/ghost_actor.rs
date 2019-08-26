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
    /// get a generic reference to ourselves
    /// will be passed into any channel process functions
    fn as_any(&mut self) -> &mut dyn Any;

    /// our parent gets a reference to the parent side of our channel
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

    /// our parent will call this process function
    fn process(&mut self) -> GhostResult<WorkWasDone> {
        // it would be awesome if this trait level could handle things like:
        //  `self.channel_self.process();`
        self.process_concrete()
    }

    /// we, as a ghost actor implement this, it will get called from
    /// process after the subconscious process items have run
    /*priv*/
    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        Ok(false.into())
    }
}

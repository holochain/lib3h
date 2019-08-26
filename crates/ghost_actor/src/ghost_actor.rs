use crate::{
    GhostCallback, GhostChannel, GhostContextChannel, GhostMessage, GhostResult, WorkWasDone,
};
use std::any::Any;

pub struct GhostParentContextChannel<
    Context,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    Error,
> {
    actor: Box<
        dyn GhostActor<
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            Error,
        >,
    >,
    channel: GhostContextChannel<
        Context,
        RequestToChild,
        RequestToChildResponse,
        RequestToParent,
        RequestToParentResponse,
        Error,
    >,
}

impl<
        Context,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
    >
    GhostParentContextChannel<
        Context,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
    >
{
    pub fn new(
        mut actor: Box<
            dyn GhostActor<
                RequestToParent,
                RequestToParentResponse,
                RequestToChild,
                RequestToChildResponse,
                Error,
            >,
        >,
        request_id_prefix: &str,
    ) -> Self {
        let channel = actor
            .take_parent_channel()
            .expect("exists")
            .as_context_channel(request_id_prefix);
        Self { actor, channel }
    }

    pub fn publish(&mut self, payload: RequestToChild) {
        self.channel.publish(payload)
    }

    pub fn request(
        &mut self,
        timeout: std::time::Duration,
        context: Context,
        payload: RequestToChild,
        cb: GhostCallback<Context, RequestToChildResponse, Error>,
    ) {
        self.channel.request(timeout, context, payload, cb)
    }

    pub fn drain_messages(
        &mut self,
    ) -> Vec<GhostMessage<RequestToParent, RequestToChild, RequestToParentResponse, Error>> {
        self.channel.drain_messages()
    }

    pub fn process(&mut self, actor: &mut dyn Any) -> GhostResult<()> {
        self.actor.process()?;
        self.channel.process(actor)?;
        Ok(())
    }
}

pub trait GhostActor<
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    E,
>
{
    /// Most of the time we don't want to keep a direct ref to the actor
    /// we want to interact through it as a channel...
    /// but we still need to call process on that actor... let us
    /// create a helper wrapper that handles that
    /*
    fn as_parent_context_channel<Context>(self) -> GhostParentContextChannel<
        Context,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        E
    > {
        let channel = self.take_parent_channel().as_context_channel();
        GhostParentContextChannel::new(self, channel)
    }*/

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

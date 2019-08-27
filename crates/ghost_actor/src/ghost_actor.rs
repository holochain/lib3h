use crate::{
    GhostCallback, GhostContextEndpoint, GhostEndpoint, GhostMessage, GhostResult, WorkWasDone,
};
use std::any::Any;

/// helper struct that merges (on the parent side) the actual child
/// GhostActor instance, with the child's ghost channel endpoint.
/// You only have to call process() on this one struct, and it provides
/// all the request / drain_messages etc functions from GhostEndpoint.
pub struct GhostParentWrapper<
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
    endpoint: GhostContextEndpoint<
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
    GhostParentWrapper<
        Context,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
    >
{
    /// wrap a GhostActor instance and it's parent channel endpoint.
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
        let endpoint = actor
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint(request_id_prefix);
        Self { actor, endpoint }
    }

    /// see GhostContextEndpoint::publish
    pub fn publish(&mut self, payload: RequestToChild) {
        self.endpoint.publish(payload)
    }

    /// see GhostContextEndpoint::request
    pub fn request(
        &mut self,
        timeout: std::time::Duration,
        context: Context,
        payload: RequestToChild,
        cb: GhostCallback<Context, RequestToChildResponse, Error>,
    ) {
        self.endpoint.request(timeout, context, payload, cb)
    }

    /// see GhostContextEndpoint::drain_messages
    pub fn drain_messages(
        &mut self,
    ) -> Vec<GhostMessage<RequestToParent, RequestToChild, RequestToParentResponse, Error>> {
        self.endpoint.drain_messages()
    }

    /// see GhostContextEndpoint::process and GhostActor::process
    pub fn process(&mut self, actor: &mut dyn Any) -> GhostResult<()> {
        self.actor.process()?;
        self.endpoint.process(actor)?;
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
    /// get a generic reference to ourselves
    /// will be passed into any endpoint process functions
    fn as_any(&mut self) -> &mut dyn Any;

    /// our parent gets a reference to the parent side of our channel
    fn take_parent_endpoint(
        &mut self,
    ) -> Option<
        GhostEndpoint<
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
        //  `self.endpoint_self.process();`
        self.process_concrete()
    }

    /// we, as a ghost actor implement this, it will get called from
    /// process after the subconscious process items have run
    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        Ok(false.into())
    }
}

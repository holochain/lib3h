use lib3h_protocol::data_types::{
    GenericResultData,
    DirectMessageData,
};
use detach::Detach;
use lib3h_protocol::data_types::SpaceData;
use core::any::Any;


use lib3h_ghost_actor::*;

// define the protocol for communicating with the real engine ghost actor
// this is a combination of lib3h client protocol and lib3h server protocol
pub enum EngineRequestToParent {
    HandleSendDirectMessage(DirectMessageData),
}

pub enum EngineRequestToParentResponse {
    HandleSendDirectMessageResult(DirectMessageData),
}

#[derive(Debug)]
pub enum EngineRequestToChild {
	JoinSpace(SpaceData),
	LeaveSpace(SpaceData),

    SendDirectMessage(DirectMessageData),
}

pub enum EngineRequestToChildResponse {
    SuccessResult(GenericResultData),
    FailureResult(GenericResultData),

    SendDirectMessageResult(DirectMessageData),
}

type EngineError = String;

pub struct GhostEngine {
    endpoint_for_parent: Option<
        GhostEndpoint<EngineRequestToChild, EngineRequestToChildResponse, EngineRequestToParent, EngineRequestToParentResponse, EngineError>
    >,
    endpoint_as_child: Detach<
        GhostContextEndpoint<
            String,
            EngineRequestToParent,
            EngineRequestToParentResponse,
            EngineRequestToChild,
            EngineRequestToChildResponse,
            EngineError,
        >,
    >,
}

impl GhostEngine {
    pub fn new() -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        Self {
            endpoint_for_parent: Some(endpoint_parent),
            endpoint_as_child: Detach::new(endpoint_self.as_context_endpoint("child")),
        }
    }
}

impl GhostActor<EngineRequestToParent, EngineRequestToParentResponse, EngineRequestToChild, EngineRequestToChildResponse, EngineError>
 for GhostEngine{
    // START BOILER PLATE--------------------------
    fn as_any(&mut self) -> &mut dyn Any {
        &mut *self
    }

    fn take_parent_endpoint(
        &mut self,
    ) -> Option<
        GhostEndpoint<EngineRequestToChild, EngineRequestToChildResponse, EngineRequestToParent, EngineRequestToParentResponse, EngineError>
    > {
        std::mem::replace(&mut self.endpoint_for_parent, None)
    }
    // END BOILER PLATE--------------------------

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
	    // START BOILER PLATE--------------------------
        // always run the endpoint process loop
        detach_run!(&mut self.endpoint_as_child, |cs| {
            cs.process(self.as_any())
        })?;
        // END BOILER PLATE--------------------------

        // when processing just print all the messages
        self.endpoint_as_child.as_mut().drain_messages().iter_mut().for_each(|msg| {
            println!("{:?}", msg.take_message().unwrap())
        });

    	Ok(true.into())
    }
}

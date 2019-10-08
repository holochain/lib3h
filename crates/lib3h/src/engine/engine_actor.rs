use crate::{engine::GhostEngine, error::Lib3hError};
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::protocol::*;

pub type ClientToLib3hMessage =
    GhostMessage<ClientToLib3h, Lib3hToClient, ClientToLib3hResponse, Lib3hError>;

pub type Lib3hToClientMessage =
    GhostMessage<Lib3hToClient, ClientToLib3h, Lib3hToClientResponse, Lib3hError>;

/// this is a generic parent wrapper for a GhostEngine.  This allows us to have
/// a mock GhostEngine for proving out the LegacyLib3h wrapper
pub type GhostEngineParentWrapper<Core, Engine, EngineError> = GhostParentWrapper<
    Core,
    Lib3hToClient,
    Lib3hToClientResponse,
    ClientToLib3h,
    ClientToLib3hResponse,
    EngineError,
    Engine,
>;

impl<'engine>
    GhostActor<
        Lib3hToClient,
        Lib3hToClientResponse,
        ClientToLib3h,
        ClientToLib3hResponse,
        Lib3hError,
    > for GhostEngine<'engine>
{
    fn take_parent_endpoint(
        &mut self,
    ) -> Option<
        GhostEndpoint<
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hToClient,
            Lib3hToClientResponse,
            Lib3hError,
        >,
    > {
        std::mem::replace(&mut self.client_endpoint, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // always run the endpoint process loop
        detach_run!(&mut self.lib3h_endpoint, |cs| { cs.process(self) })?;

        // process any messages from the client to us
        let mut did_work = false;
        for msg in self.lib3h_endpoint.as_mut().drain_messages() {
            self.handle_msg_from_client(msg)?;
            did_work = true;
        }

        // Process network layer
        did_work = did_work || self.process_multiplexer()?;

        // Process the space layer
        did_work = did_work || self.process_space_gateways()?;

        // Done
        // trace!("({}).process_concrete() did_work = {}", self.name, did_work);
        Ok(did_work.into())
    }
}

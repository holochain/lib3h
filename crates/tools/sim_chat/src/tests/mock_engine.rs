use lib3h_ghost_actor::prelude::WorkWasDone;
use lib3h_protocol::{
    protocol::{ClientToLib3h, ClientToLib3hResponse, Lib3hToClient, Lib3hToClientResponse},
    data_types::ConnectedData,
};
use lib3h::{
    error::Lib3hError,
    engine::ghost_engine::{ClientToLib3hMessage}
};
use detach::Detach;
use lib3h_ghost_actor::{
	GhostActor,
	GhostEndpoint,
	GhostResult,
	GhostContextEndpoint,
    GhostCanTrack,
    GhostError,
    create_ghost_channel,
};

pub struct MockEngine<'engine> {
    lib3h_endpoint: Detach<
        GhostContextEndpoint<
            MockEngine<'engine>,
            (),
            Lib3hToClient,
            Lib3hToClientResponse,
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hError,
        >,
    >,
    client_endpoint: Option<
        GhostEndpoint<
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hToClient,
            Lib3hToClientResponse,
            Lib3hError,
        >,
    >,
}

impl GhostActor<
    Lib3hToClient,
    Lib3hToClientResponse,
    ClientToLib3h,  
    ClientToLib3hResponse,
    Lib3hError,
> for MockEngine<'_> {
    // START BOILER PLATE--------------------------
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
    // END BOILER PLATE--------------------------

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // START BOILER PLATE--------------------------
        // always run the endpoint process loop
        detach_run!(&mut self.lib3h_endpoint, |cs| { cs.process(self) })?;
        // END BOILER PLATE--------------------------

        for msg in self.lib3h_endpoint.as_mut().drain_messages() {
            self.handle_msg_from_client(msg)?;
        }

        Ok(true.into())
    }
}

impl MockEngine<'_> {

    pub fn new() -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        Self {
            client_endpoint: Some(endpoint_parent),
            lib3h_endpoint: Detach::new(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix("mock-engine")
                    .build(),
            ),
        }
    }

        /// Process any Client events or requests
    fn handle_msg_from_client(&mut self, mut msg: ClientToLib3hMessage) -> Result<(), GhostError> {
        match msg.take_message().expect("exists") {
            ClientToLib3h::Connect(data) => {
                msg.respond(Ok(ClientToLib3hResponse::ConnectResult(ConnectedData {
                    request_id: data.request_id,
                    uri: data.peer_uri,
                })))
            }
            ClientToLib3h::JoinSpace(_data) => {
                msg.respond(Ok(ClientToLib3hResponse::JoinSpaceResult))
            }
            ClientToLib3h::LeaveSpace(_data) => {
                msg.respond(Ok(ClientToLib3hResponse::LeaveSpaceResult))
            }
            ClientToLib3h::SendDirectMessage(_data) => {
                Ok(())
            }
            ClientToLib3h::PublishEntry(_data) => {
                Ok(())
            },
            ClientToLib3h::HoldEntry(_data) => {
                Ok(())
            },
            ClientToLib3h::QueryEntry(_data) => {
                Ok(())
            }
            _ => panic!("{:?} not implemented", msg),
        }
    }
}

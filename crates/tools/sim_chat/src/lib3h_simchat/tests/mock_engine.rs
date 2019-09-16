use detach::{detach_run, Detach};
use lib3h::{engine::CanAdvertise, error::Lib3hError};

use lib3h::engine::engine_actor::ClientToLib3hMessage;
use lib3h_protocol::protocol::{
    ClientToLib3h, ClientToLib3hResponse, Lib3hToClient, Lib3hToClientResponse,
};
use lib3h_zombie_actor::{
    create_ghost_channel, GhostActor, GhostCanTrack, GhostContextEndpoint, GhostEndpoint,
    GhostError, GhostResult, WorkWasDone,
};
use url::Url;

/**
 *  This is an engine that exists purely for testing the SimChat actor.
 *  It responds with success to any ClientToLib3h messages sent to it but does not send a response
 *  
 *  To test sending Lib3hToClient messages it can hold a crossbeam receiver and will route any messages
 *  it receives to SimChat.
 */

pub struct MockEngine<'engine> {
    lib3h_endpoint: Detach<
        GhostContextEndpoint<
            MockEngine<'engine>,
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
    // reciever: Option<crossbeam_channel::Receiver<Lib3hToClient>>,
}

impl
    GhostActor<
        Lib3hToClient,
        Lib3hToClientResponse,
        ClientToLib3h,
        ClientToLib3hResponse,
        Lib3hError,
    > for MockEngine<'_>
{
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
            // reciever: None,
        }
    }

    // pub fn new_with_sender() -> (Self, crossbeam_channel::Sender<Lib3hToClient>) {
    //     let mut engine = MockEngine::new();
    //     let (s, r)  = crossbeam_channel::unbounded();
    //     engine.reciever = Some(r);
    //     (engine, s)
    // }

    /// Process any Client events or requests
    fn handle_msg_from_client(&mut self, mut msg: ClientToLib3hMessage) -> Result<(), GhostError> {
        match msg.take_message().expect("exists") {
            ClientToLib3h::Bootstrap(_) => msg.respond(Ok(ClientToLib3hResponse::BootstrapSuccess)),
            ClientToLib3h::JoinSpace(_data) => {
                msg.respond(Ok(ClientToLib3hResponse::JoinSpaceResult))
            }
            ClientToLib3h::LeaveSpace(_data) => {
                msg.respond(Ok(ClientToLib3hResponse::LeaveSpaceResult))
            }
            ClientToLib3h::SendDirectMessage(_data) => Ok(()),
            ClientToLib3h::PublishEntry(_data) => Ok(()),
            ClientToLib3h::HoldEntry(_data) => Ok(()),
            ClientToLib3h::QueryEntry(_data) => Ok(()),
            _ => panic!("{:?} not implemented", msg),
        }
    }
}

impl CanAdvertise for MockEngine<'_> {
    fn advertise(&self) -> Url {
        Url::parse("ws://mock_peer_url").unwrap()
    }
}

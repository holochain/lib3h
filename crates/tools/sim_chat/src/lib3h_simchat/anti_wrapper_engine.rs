

use detach::{detach_run, Detach};
use lib3h::{engine::CanAdvertise, error::Lib3hError};


use lib3h_protocol::protocol::{
    ClientToLib3h, ClientToLib3hResponse, Lib3hToClient, Lib3hToClientResponse,
};
use lib3h_protocol::{
    protocol_client::Lib3hClientProtocol,
    network_engine::NetworkEngine,
};
use lib3h_zombie_actor::{
    GhostActor, GhostCanTrack, GhostContextEndpoint, GhostEndpoint, GhostResult, WorkWasDone, GhostMessage,
    GhostError, RequestId,
};
use url::Url;
use std::convert::TryFrom;
use std::collections::HashMap;


/// This is a ghost actor engine that wraps non-ghost implementors of NetworkEngine (e.g. old Lib3h, Sim1h)
pub struct AntiWrapperEngine<'engine, T: NetworkEngine> {
    client_endpoint: Option<
        GhostEndpoint<
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hToClient,
            Lib3hToClientResponse,
            Lib3hError,
        >,
    >,
    lib3h_endpoint: Detach<
        GhostContextEndpoint<
            AntiWrapperEngine<'engine, T>,
            Lib3hToClient,
            Lib3hToClientResponse,
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hError,
        >,
    >,
    pending_requests_to_client: HashMap<RequestId, GhostMessage<ClientToLib3h, Lib3hToClient, ClientToLib3hResponse, GhostError>>,
    network_engine: T,
}

impl<T: NetworkEngine>
    GhostActor<
        Lib3hToClient,
        Lib3hToClientResponse,
        ClientToLib3h,
        ClientToLib3hResponse,
        Lib3hError,
    > for AntiWrapperEngine<'_, T>
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

        // grab any messages FROM THE ENGINE INSTANCE if work was done
        // These will be Lib3hServerProtocol types which includes server->client requests 
        // as well as responses to client->server requests
        // convert these to the Lib3hToClient or ClientToLib3hResponse and send over the client endpoint
        if let Ok((true, from_engine_messages)) = self.network_engine.process() {
            for msg_from_engine in from_engine_messages {
                if let Ok(_request) = Lib3hToClient::try_from(msg_from_engine.clone()) {
                    // send the request to  the client
                    // let request_ghost_message = 
                    // hold on to the request so we can send back a response later

                } else if let Ok(response) = ClientToLib3hResponse::try_from(msg_from_engine) {
                    // see if this is the response to a pending request and send it back to that
                    // let request_id = 
                    if let Some(ghost_request) = self.pending_requests_to_client.remove(&RequestId("".into())) {
                        ghost_request.respond(Ok(response)).ok();
                    }
                } else {
                    panic!("anti-wrapper engine received a message from engine that could not be translated")
                }
            }
        }

    	// Messages FROM THE CLIENT will be ClientToLib3h messages
        // Convert these to Lib3hClientProtocol and send over the sender 
        for mut msg in self.lib3h_endpoint.as_mut().drain_messages() {
            let msg_to_engine = Lib3hClientProtocol::from(msg.take_message().expect("exists"));
            if self.network_engine.post(msg_to_engine).is_ok() {
                // manually send any responses back to the client after post success
                // let msg_to_engine_response = Lib3hClientProtocol::
                // self.network_engine.post(msg_to_engine_response).ok();
            }
        }

        Ok(true.into())
    }
}

impl<T: NetworkEngine> CanAdvertise for AntiWrapperEngine<'_, T> {
    fn advertise(&self) -> Url {
        Url::parse("ws://mock_peer_url").unwrap()
    }
}

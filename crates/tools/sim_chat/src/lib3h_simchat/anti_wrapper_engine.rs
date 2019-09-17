

use lib3h_protocol::protocol_server::Lib3hServerProtocol;
use lib3h_tracing::test_span;
use detach::{detach_run, Detach};
use lib3h::{engine::CanAdvertise, error::Lib3hError};


use lib3h_protocol::protocol::{
    ClientToLib3h, ClientToLib3hResponse, Lib3hToClient, Lib3hToClientResponse,
};
use lib3h_protocol::{
    protocol_client::Lib3hClientProtocol,
    network_engine::NetworkEngine,
    data_types::*,
};
use lib3h_zombie_actor::{
    create_ghost_channel,
    GhostActor, GhostCanTrack, GhostContextEndpoint, GhostEndpoint, GhostResult, WorkWasDone, GhostMessage, RequestId, GhostCallbackData::Response,
};
use url::Url;
use std::convert::TryFrom;
use std::collections::HashMap;


/// This is a ghost actor engine that wraps non-ghost implementors of NetworkEngine (e.g. old Lib3h, Sim1h)
pub struct AntiWrapperEngine<'engine, T: 'static + NetworkEngine> {
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
    pending_requests_to_client: HashMap<RequestId, GhostMessage<ClientToLib3h, Lib3hToClient, ClientToLib3hResponse, Lib3hError>>,
    network_engine: T,
}

impl<'engine, T: 'static + NetworkEngine> AntiWrapperEngine<'engine, T> {
    pub fn new(network_engine: T) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        Self {
            client_endpoint: Some(endpoint_parent),
            lib3h_endpoint: Detach::new(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix("mock-engine")
                    .build(),
            ),
            network_engine,
            pending_requests_to_client: HashMap::new(),
        }
    }
}

impl<T: NetworkEngine>
    GhostActor<
        Lib3hToClient,
        Lib3hToClientResponse,
        ClientToLib3h,
        ClientToLib3hResponse,
        Lib3hError,
    > for AntiWrapperEngine<'static, T>
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
                if let Ok(request) = Lib3hToClient::try_from(msg_from_engine.clone()) {
                    // send the request to  the client
                    self.lib3h_endpoint.request(test_span(""), request, Box::new(|engine, callback_data| {
                        // send the client response back to the engine if it was a success
                        if let Response(Ok(success_message)) = callback_data {
                            engine.network_engine.post(success_message.into()).ok();
                        }
                        Ok(())
                    })).ok();

                } 
                else if let Ok(response) = ClientToLib3hResponse::try_from(msg_from_engine.clone()) {
                    // see if this is the response to a pending request and send it back to that
                    if let Some(request_id) = msg_from_engine.request_id() {
                        if let Some(ghost_request) = self.pending_requests_to_client.remove(&request_id) {
                            ghost_request.respond(Ok(response)).ok();
                        }
                    }
                } 
                else {
                    panic!("anti-wrapper engine received a message from engine that could not be translated")
                }
            }
        }

    	// Messages FROM THE CLIENT will be ClientToLib3h messages
        // Convert these to Lib3hClientProtocol and send over the sender 
        for mut msg in self.lib3h_endpoint.as_mut().drain_messages() {
            let msg_to_engine = Lib3hClientProtocol::from(msg.take_message().expect("exists"));
            // send the converted message onward to the network engine
            self.network_engine.post(msg_to_engine.clone()).ok();
            // register this locally to repond to it later (if it has a request id)
            if let Some(request_id) = msg.request_id() {
                self.pending_requests_to_client.insert(request_id, msg);
            }
        }

        Ok(true.into())
    }
}

impl<T: NetworkEngine> CanAdvertise for AntiWrapperEngine<'_, T> {
    fn advertise(&self) -> Url {
        self.network_engine.advertise()
    }
}

trait RequestIdGetable {
    fn request_id(&self) -> Option<RequestId>;
}

impl RequestIdGetable for Lib3hServerProtocol {
    fn request_id(&self) -> Option<RequestId> {
        match self {
            Lib3hServerProtocol::SuccessResult(GenericResultData{request_id, ..})
            | Lib3hServerProtocol::FailureResult(GenericResultData{request_id, ..})
            | Lib3hServerProtocol::Connected(ConnectedData{request_id, ..})
            | Lib3hServerProtocol::SendDirectMessageResult(DirectMessageData{request_id, ..})
            | Lib3hServerProtocol::HandleSendDirectMessage(DirectMessageData{request_id, ..})
            | Lib3hServerProtocol::FetchEntryResult(FetchEntryResultData{request_id, ..})
            | Lib3hServerProtocol::HandleFetchEntry(FetchEntryData{request_id, ..})
            | Lib3hServerProtocol::HandleStoreEntryAspect(StoreEntryAspectData{request_id, ..})
            | Lib3hServerProtocol::HandleDropEntry(DropEntryData{request_id, ..})
            | Lib3hServerProtocol::HandleQueryEntry(QueryEntryData{request_id, ..})
            | Lib3hServerProtocol::QueryEntryResult(QueryEntryResultData{request_id, ..})
            | Lib3hServerProtocol::HandleGetAuthoringEntryList(GetListData{request_id, ..})
            | Lib3hServerProtocol::HandleGetGossipingEntryList(GetListData{request_id, ..})
             => {
                Some(RequestId(request_id.clone()))
            }
            _ => None
        }
    }
}

// #[cfg(test)]
// pub mod tests {

//     use lib3h_protocol::{
//         error::Lib3hProtocolResult,
//         DidWork,
//     };
    
//     use super::*;

//     struct MockNetworkEngine{}

//     impl NetworkEngine for MockNetworkEngine {

//         fn post(&mut self, _data: Lib3hClientProtocol) -> Lib3hProtocolResult<()> {
//             Ok(())
//         }

//         fn process(&mut self) -> Lib3hProtocolResult<(DidWork, Vec<Lib3hServerProtocol>)> {
//             Ok((true, Vec::new()))
//         }

//         fn advertise(&self) -> Url {
//             Url::parse("ws://test").unwrap()
//         }

//         fn name(&self) -> String {
//             String::from("mock-engine")
//         }
//     }

//     #[test]
//     fn test_round_trip_from_client() {
//         let network_engine = MockNetworkEngine{};
//         let anti_wrapper = AntiWrapperEngine::new(network_engine);

//         let mut parent_endpoint: GhostContextEndpoint<(), _, _, _, _, _> = anti_wrapper
//             .take_parent_endpoint()
//             .expect("Could not get parent endpoint")
//             .as_context_endpoint_builder()
//             .request_id_prefix("parent")
//             .build();

//         let test_message = ClientToLib3h::JoinSpace(SpaceData {
//             agent_id: Address::from(""),
//             space_address: Address::from(""),
//             request_id: String::from("0"),
//         });

//         parent_endpoint.publish(test_span(""), test_message.expect("Could not publish a message to the wrapper")
//     }
// }

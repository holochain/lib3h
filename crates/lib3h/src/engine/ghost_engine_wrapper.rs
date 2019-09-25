use crate::{
    engine::{engine_actor::GhostEngineParentWrapper, CanAdvertise, GhostEngine},
    error::*,
    track::Tracker,
};

use std::convert::TryInto;

use detach::Detach;
use holochain_tracing::Span;
use lib3h_ghost_actor::*;
use lib3h_protocol::{
    data_types::{ConnectedData, GenericResultData, Opaque},
    error::{ErrorKind, Lib3hProtocolError, Lib3hProtocolResult},
    protocol::*,
    protocol_client::*,
    protocol_server::*,
    uri::Lib3hUri,
    Address, DidWork,
};
pub type WrappedGhostLib3h = LegacyLib3h<GhostEngine<'static>, Lib3hError>;

/// A wrapper for talking to lib3h using the legacy Lib3hClient/Server enums
#[allow(dead_code)]
pub struct LegacyLib3h<Engine, EngineError: 'static + std::fmt::Debug>
where
    Engine: GhostActor<
        Lib3hToClient,
        Lib3hToClientResponse,
        ClientToLib3h,
        ClientToLib3hResponse,
        EngineError,
    >,
{
    engine: Detach<GhostEngineParentWrapper<LegacyLib3h<Engine, EngineError>, Engine, EngineError>>,
    #[allow(dead_code)]
    name: String,
    client_request_responses: Vec<Lib3hServerProtocol>,
    tracker:
        Tracker<GhostMessage<Lib3hToClient, ClientToLib3h, Lib3hToClientResponse, EngineError>>,
    req_id_map: std::collections::HashMap<String, String>,
}

fn server_failure(
    err: String,
    request_id: String,
    space_address: Address,
    to_agent_id: Address,
) -> Lib3hServerProtocol {
    let failure_data = GenericResultData {
        request_id,
        space_address,
        to_agent_id,
        result_info: err.as_bytes().into(),
    };
    Lib3hServerProtocol::FailureResult(failure_data)
}

fn server_success(
    request_id: String,
    space_address: Address,
    to_agent_id: Address,
) -> Lib3hServerProtocol {
    let success_data = GenericResultData {
        request_id,
        space_address,
        to_agent_id,
        result_info: Opaque::new(),
    };
    Lib3hServerProtocol::SuccessResult(success_data)
}

#[allow(dead_code)]
impl<Engine: 'static, EngineError: 'static + std::fmt::Debug> LegacyLib3h<Engine, EngineError>
where
    Engine: GhostActor<
            Lib3hToClient,
            Lib3hToClientResponse,
            ClientToLib3h,
            ClientToLib3hResponse,
            EngineError,
        > + CanAdvertise,
    EngineError: ToString,
{
    pub fn new(name: &str, engine: Engine) -> Self {
        LegacyLib3h {
            engine: Detach::new(GhostParentWrapper::new(engine, name)),
            name: name.into(),
            client_request_responses: Vec::new(),
            tracker: Tracker::new("client_to_lib3_response_", 2000),
            req_id_map: std::collections::HashMap::new(),
        }
    }

    fn make_callback(
        request_id: String,
        space_addr: Address,
        agent: Address,
    ) -> GhostCallback<LegacyLib3h<Engine, EngineError>, ClientToLib3hResponse, EngineError> {
        Box::new(
            |me: &mut LegacyLib3h<Engine, EngineError>,
             response: GhostCallbackData<ClientToLib3hResponse, EngineError>| {
                match response {
                    GhostCallbackData::Response(Ok(rsp)) => {
                        let response = match rsp {
                            ClientToLib3hResponse::BootstrapSuccess => {
                                Lib3hServerProtocol::Connected(ConnectedData {
                                    request_id,
                                    uri: Lib3hUri::with_undefined(), // client should have this already deprecated
                                })
                            }
                            ClientToLib3hResponse::JoinSpaceResult => {
                                server_success(request_id.clone(), space_addr, agent)
                            }
                            ClientToLib3hResponse::LeaveSpaceResult => {
                                server_success(request_id.clone(), space_addr, agent)
                            }
                            _ => rsp.into(),
                        };
                        me.client_request_responses.push(response)
                    }
                    GhostCallbackData::Response(Err(e)) => {
                        me.client_request_responses.push(server_failure(
                            e.to_string(),
                            request_id.clone(),
                            space_addr,
                            agent,
                        ));
                    }
                    GhostCallbackData::Timeout(bt) => {
                        me.client_request_responses.push(server_failure(
                            format!("Request timed out: {:?}", bt),
                            request_id,
                            space_addr,
                            agent,
                        ));
                    }
                };
                Ok(())
            },
        )
    }

    /// Add incoming Lib3hClientProtocol message in FIFO
    pub fn post(&mut self, client_msg: Lib3hClientProtocol) -> Lib3hProtocolResult<()> {
        let (request_id, space_addr, agent_id) = match &client_msg {
            Lib3hClientProtocol::Connect(data) => (
                data.request_id.to_string(),
                "bogus_address".into(),
                "bogus_agent".into(),
            ),
            Lib3hClientProtocol::JoinSpace(data) => (
                data.request_id.to_string(),
                data.space_address.clone(),
                data.agent_id.clone(),
            ),
            Lib3hClientProtocol::LeaveSpace(data) => (
                data.request_id.to_string(),
                data.space_address.clone(),
                data.agent_id.clone(),
            ),
            Lib3hClientProtocol::SendDirectMessage(data) => (
                data.request_id.to_string(),
                data.space_address.clone(),
                data.to_agent_id.clone(),
            ),
            Lib3hClientProtocol::FetchEntry(data) => (
                data.request_id.to_string(),
                data.space_address.clone(),
                data.provider_agent_id.clone(),
            ),
            Lib3hClientProtocol::QueryEntry(data) => (
                data.request_id.to_string(),
                data.space_address.clone(),
                data.requester_agent_id.clone(),
            ),
            Lib3hClientProtocol::HandleSendDirectMessageResult(data) => (
                data.request_id.to_string(),
                data.space_address.clone(),
                "".into(),
            ), // agent id is deprecated here, client should know.
            Lib3hClientProtocol::HandleFetchEntryResult(data) => (
                data.request_id.to_string(),
                data.space_address.clone(),
                data.provider_agent_id.clone(),
            ),
            Lib3hClientProtocol::HandleQueryEntryResult(data) => (
                data.request_id.to_string(),
                data.space_address.clone(),
                "".into(),
            ), // agent id is deprecated here, client should know.
            Lib3hClientProtocol::HandleGetAuthoringEntryListResult(data) => (
                data.request_id.to_string(),
                data.space_address.clone(),
                data.provider_agent_id.clone(),
            ),
            Lib3hClientProtocol::HandleGetGossipingEntryListResult(data) => (
                data.request_id.to_string(),
                data.space_address.clone(),
                data.provider_agent_id.clone(),
            ),
            Lib3hClientProtocol::PublishEntry(data) => (
                "".to_string(),
                data.space_address.clone(),
                data.provider_agent_id.clone(),
            ),
            Lib3hClientProtocol::HoldEntry(data) => (
                "".to_string(),
                data.space_address.clone(),
                data.provider_agent_id.clone(),
            ),
            _ => unimplemented!(),
        };

        let maybe_client_to_lib3h: Result<ClientToLib3h, _> = client_msg.clone().try_into();
        if let Ok(client_to_lib3h) = maybe_client_to_lib3h {
            let result = if request_id == "" {
                self.engine.publish(Span::fixme(), client_to_lib3h)
            } else {
                self.engine.request(
                    Span::fixme(),
                    client_to_lib3h,
                    LegacyLib3h::make_callback(request_id.to_string(), space_addr, agent_id),
                )
            };
            result.map_err(|e| Lib3hProtocolError::new(ErrorKind::Other(e.to_string())))
        } else {
            // TODO Handle errors better here!
            let lib3h_to_client_response: Lib3hToClientResponse =
                client_msg.clone().try_into().unwrap();
            let maybe_ghost_message: Option<GhostMessage<_, _, Lib3hToClientResponse, _>> =
                self.tracker.remove(request_id.as_str());
            let ghost_mesage = maybe_ghost_message.ok_or_else(|| {
                Lib3hProtocolError::new(ErrorKind::Other(format!(
                    "No ghost message for request: {:?}",
                    request_id.as_str()
                )))
            })?;

            let resp;
            if let Lib3hToClientResponse::HandleSendDirectMessageResult(mut data) =
                lib3h_to_client_response.clone()
            {
                trace!("IS CASE: {:?}", lib3h_to_client_response);
                let result = self.req_id_map.remove(&request_id);
                if let Some(req_id) = result {
                    trace!("REPLACE req_id {:?} with {:?}", data.request_id, req_id);
                    data.request_id = req_id.clone();
                }
                resp = Lib3hToClientResponse::HandleSendDirectMessageResult(data);
            } else {
                resp = lib3h_to_client_response.clone();
            }

            ghost_mesage
                .respond(Ok(resp))
                .map_err(|e| Lib3hProtocolError::new(ErrorKind::Other(e.to_string())))
        }
    }

    /// Process Lib3hClientProtocol message inbox and
    /// output a list of Lib3hServerProtocol messages for Core to handle
    pub fn process(&mut self) -> Lib3hProtocolResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        trace!("[legacy engine] process");

        let did_work = detach_run!(&mut self.engine, |engine| engine.process(self))
            .map_err(|e| Lib3hProtocolError::new(ErrorKind::Other(e.to_string())))?;
        // get any "server" messages that came as responses to the client requests
        let mut responses: Vec<Lib3hServerProtocol> =
            self.client_request_responses.drain(..).collect();

        for mut msg in self.engine.as_mut().drain_messages() {
            let tracker_request_id = self.tracker.reserve();

            let lib3h_to_client_msg: Lib3hToClient = msg.take_message().expect("exists");

            trace!(
                "[legacy engine] reserve {:?} for {:?}",
                tracker_request_id,
                lib3h_to_client_msg
            );

            self.tracker.set(tracker_request_id.as_str(), Some(msg));
            let lib3h_server_protocol_msg: Lib3hServerProtocol =
                self.inject_request_id(tracker_request_id.clone(), lib3h_to_client_msg.into());
            responses.push(lib3h_server_protocol_msg);
        }

        Ok((*did_work, responses))
    }

    pub fn advertise(&self) -> Lib3hUri {
        self.engine.as_ref().as_ref().advertise()
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    fn inject_request_id(
        &mut self,
        request_id: String,
        mut msg: Lib3hServerProtocol,
    ) -> Lib3hServerProtocol {
        match &mut msg {
            Lib3hServerProtocol::Connected(data) => data.request_id = request_id,
            Lib3hServerProtocol::FetchEntryResult(data) => data.request_id = request_id,
            Lib3hServerProtocol::HandleFetchEntry(data) => data.request_id = request_id,
            Lib3hServerProtocol::HandleStoreEntryAspect(data) => data.request_id = request_id,
            Lib3hServerProtocol::HandleDropEntry(data) => data.request_id = request_id,
            Lib3hServerProtocol::HandleQueryEntry(data) => data.request_id = request_id,
            Lib3hServerProtocol::HandleGetAuthoringEntryList(data) => data.request_id = request_id,
            Lib3hServerProtocol::HandleGetGossipingEntryList(data) => data.request_id = request_id,
            Lib3hServerProtocol::HandleSendDirectMessage(data) => {
                self.req_id_map
                    .insert(request_id.clone(), data.request_id.clone());
                data.request_id = request_id
            }
            msg => error!("[inject_request_id] CONVERT ME: {:?}", msg),
        }
        msg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use holochain_tracing::test_span;
    use lib3h_protocol::data_types::*;
    use url::Url;

    type EngineError = String;

    pub struct MockGhostEngine {
        client_endpoint: Option<
            GhostEndpoint<
                ClientToLib3h,
                ClientToLib3hResponse,
                Lib3hToClient,
                Lib3hToClientResponse,
                EngineError,
            >,
        >,
        lib3h_endpoint: Detach<
            GhostContextEndpoint<
                MockGhostEngine,
                Lib3hToClient,
                Lib3hToClientResponse,
                ClientToLib3h,
                ClientToLib3hResponse,
                EngineError,
            >,
        >,
    }

    impl CanAdvertise for MockGhostEngine {
        fn advertise(&self) -> Lib3hUri {
            Lib3hUri::with_memory("fixme")
        }
    }

    impl MockGhostEngine {
        pub fn new() -> Self {
            let (endpoint_parent, endpoint_self) = create_ghost_channel();
            Self {
                client_endpoint: Some(endpoint_parent),
                lib3h_endpoint: Detach::new(
                    endpoint_self
                        .as_context_endpoint_builder()
                        .request_id_prefix("engine")
                        .build(),
                ),
            }
        }
    }

    impl
        GhostActor<
            Lib3hToClient,
            Lib3hToClientResponse,
            ClientToLib3h,
            ClientToLib3hResponse,
            EngineError,
        > for MockGhostEngine
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
                EngineError,
            >,
        > {
            std::mem::replace(&mut self.client_endpoint, None)
        }
        // END BOILER PLATE--------------------------

        fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
            // START BOILER PLATE--------------------------
            // always run the endpoint process loop
            detach_run!(&mut self.lib3h_endpoint, |cs| cs.process(self))?;
            // END BOILER PLATE--------------------------

            for msg in self.lib3h_endpoint.as_mut().drain_messages() {
                self.handle_msg_from_client(msg)?;
            }

            Ok(true.into())
        }
    }

    impl MockGhostEngine {
        fn handle_msg_from_client(
            &mut self,
            mut msg: GhostMessage<ClientToLib3h, Lib3hToClient, ClientToLib3hResponse, EngineError>,
        ) -> Result<(), EngineError> {
            let result = match msg.take_message().expect("exists") {
                ClientToLib3h::Bootstrap(_data) => {
                    // pretend the connection request failed
                    msg.respond(Err("connection failed!".to_string()))
                }
                ClientToLib3h::JoinSpace(_data) => {
                    // pretend the request succeeded
                    msg.respond(Ok(ClientToLib3hResponse::JoinSpaceResult))
                }
                _ => panic!("{:?} not implemented", msg),
            };
            result.map_err(|e| e.to_string())
        }

        /// create a fake lib3h publish event
        pub fn inject_lib3h_publish(&mut self, msg: Lib3hToClient) {
            let _ = self
                .lib3h_endpoint
                .publish(test_span("inject_lib3h_event"), msg);
        }

        /// create a fake lib3h request
        pub fn inject_lib3h_request(&mut self, msg: Lib3hToClient) {
            let f: GhostCallback<_, _, _> = Box::new(|_user_data, cb_data| {
                debug!("inject_lib3h_request: {:?}", cb_data);
                Ok(())
            });

            let _ = self
                .lib3h_endpoint
                .request(test_span("inject_lib3h_request"), msg, f);
        }
    }

    struct MockCore {
        //    state: String,
    }

    #[test]
    fn test_ghost_engine_wrapper() {
        let mut _core = MockCore {
    //        state: "".to_string(),
        };

        // create the legacy lib3h engine wrapper
        let mut legacy: LegacyLib3h<MockGhostEngine, EngineError> =
            LegacyLib3h::new("core", MockGhostEngine::new());

        let data = ConnectData {
            request_id: "foo_request_id".into(),
            peer_location: Url::parse("mocknet://t1").expect("can parse url").into(),
            network_id: "fake_id".to_string(),
        };

        assert!(legacy.post(Lib3hClientProtocol::Connect(data)).is_ok());
        // process via the wrapper
        let result = legacy.process();

        // The mock engine allways returns failure on connect requests
        assert_eq!(
            "Ok((true, [FailureResult(GenericResultData { request_id: \"foo_request_id\", space_address: HashString(\"bogus_address\"), to_agent_id: HashString(\"bogus_agent\"), result_info: \"connection failed!\" })]))",
            format!("{:?}", result)
        );

        let data = SpaceData {
            request_id: "bar_request_id".into(),
            space_address: "fake_space_address".into(),
            agent_id: "fake_id".into(),
        };
        assert!(legacy.post(Lib3hClientProtocol::JoinSpace(data)).is_ok());
        // process via the wrapper
        let result = legacy.process();

        // The mock engine allways returns success on Join requests
        assert_eq!(
            "Ok((true, [SuccessResult(GenericResultData { request_id: \"bar_request_id\", space_address: HashString(\"fake_space_address\"), to_agent_id: HashString(\"fake_id\"), result_info: \"\" })]))",
            format!("{:?}", result)
        );

        detach_run!(&mut legacy.engine, |l| l.as_mut().inject_lib3h_request(
            Lib3hToClient::HandleGetGossipingEntryList(GetListData {
                space_address: "fake_space_address".into(),
                provider_agent_id: "fake_id".into(),
                request_id: "post_gossip_req_id".into(),
            })
        ));

        let result = legacy.process();
        let requestid = match result {
            Ok((true, responses)) => {
                assert_eq!(responses.len(), 1);
                match &responses[0] {
                    Lib3hServerProtocol::HandleGetGossipingEntryList(data) => {
                        data.request_id.clone()
                    }
                    _ => "bogus".to_string(),
                }
            }
            _ => "bogus".to_string(),
        };
        assert_eq!("client_to_lib3_response", requestid.split_at(23).0);

        legacy
            .post(Lib3hClientProtocol::HandleGetGossipingEntryListResult(
                EntryListData {
                    space_address: "fake_space_address".into(),
                    provider_agent_id: "fake_id".into(),
                    request_id: requestid,
                    address_map: std::collections::HashMap::new(),
                },
            ))
            .unwrap();
        let result = legacy.process();

        assert_eq!("Ok((true, []))", format!("{:?}", result));

        detach_run!(&mut legacy.engine, |l| l.as_mut().inject_lib3h_publish(
            Lib3hToClient::Disconnected(DisconnectedData {
                network_id: "some_network_id".into()
            })
        ));

        let result = legacy.process();

        assert_eq!(
            "Ok((true, [Disconnected(DisconnectedData { network_id: \"some_network_id\" })]))",
            format!("{:?}", result)
        );
    }
}

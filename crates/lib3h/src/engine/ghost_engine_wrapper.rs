use crate::engine::ghost_engine::GhostEngineParentWrapper;
use detach::Detach;
use lib3h_ghost_actor::*;
use lib3h_protocol::{
    data_types::GenericResultData,
    error::{ErrorKind, Lib3hProtocolError, Lib3hProtocolResult},
    protocol::*,
    protocol_client::*,
    protocol_server::*,
    DidWork,
};
use lib3h_tracing::Lib3hSpan;

/// A wrapper for talking to lib3h using the legacy Lib3hClient/Server enums
#[allow(dead_code)]
struct LegacyLib3h<Engine, EngineError: 'static>
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
}

fn server_failure(err: String, request_id: String) -> Lib3hServerProtocol {
    let failure_data = GenericResultData {
        request_id,
        space_address: "space_addr".into(),
        to_agent_id: "to_agent_id".into(),
        result_info: err.as_bytes().into(),
    };
    Lib3hServerProtocol::FailureResult(failure_data)
}

fn server_success(request_id: String) -> Lib3hServerProtocol {
    let failure_data = GenericResultData {
        request_id,
        space_address: "space_addr".into(),
        to_agent_id: "to_agent_id".into(),
        result_info: vec![].into(),
    };
    Lib3hServerProtocol::FailureResult(failure_data)
}

#[allow(dead_code)]
impl<Engine: 'static, EngineError: 'static> LegacyLib3h<Engine, EngineError>
where
    Engine: GhostActor<
        Lib3hToClient,
        Lib3hToClientResponse,
        ClientToLib3h,
        ClientToLib3hResponse,
        EngineError,
    >,
    EngineError: ToString,
{
    pub fn new(name: &str, engine: Engine) -> Self {
        LegacyLib3h {
            engine: Detach::new(GhostParentWrapper::new(engine, name)),
            name: name.into(),
            client_request_responses: Vec::new(),
        }
    }

    fn make_callback(
        request_id: String,
    ) -> GhostCallback<LegacyLib3h<Engine, EngineError>, ClientToLib3hResponse, EngineError> {
        Box::new(
            |me: &mut LegacyLib3h<Engine, EngineError>,
             response: GhostCallbackData<ClientToLib3hResponse, EngineError>| {
                match response {
                    GhostCallbackData::Response(Ok(rsp)) => {
                        let response = match rsp {
                            ClientToLib3hResponse::JoinSpaceResult => {
                                server_success(request_id.clone())
                            }
                            ClientToLib3hResponse::LeaveSpaceResult => {
                                server_success(request_id.clone())
                            }
                            _ => rsp.into(),
                        };
                        me.client_request_responses.push(response)
                    }
                    GhostCallbackData::Response(Err(e)) => {
                        me.client_request_responses
                            .push(server_failure(e.to_string(), request_id.clone()));
                    }
                    GhostCallbackData::Timeout => {
                        me.client_request_responses
                            .push(server_failure("Request timed out".into(), request_id));
                    }
                };
                Ok(())
            },
        )
    }

    /// Add incoming Lib3hClientProtocol message in FIFO
    fn post(&mut self, client_msg: Lib3hClientProtocol) -> Lib3hProtocolResult<()> {
        let request_id: String = match &client_msg {
            Lib3hClientProtocol::Connect(data) => &data.request_id,
            Lib3hClientProtocol::JoinSpace(data) => &data.request_id,
            Lib3hClientProtocol::LeaveSpace(data) => &data.request_id,
            Lib3hClientProtocol::SendDirectMessage(data) => &data.request_id,
            Lib3hClientProtocol::FetchEntry(data) => &data.request_id,
            Lib3hClientProtocol::QueryEntry(data) => &data.request_id,
            Lib3hClientProtocol::HandleSendDirectMessageResult(data) => &data.request_id,
            Lib3hClientProtocol::HandleFetchEntryResult(data) => &data.request_id,
            Lib3hClientProtocol::HandleQueryEntryResult(data) => &data.request_id,
            Lib3hClientProtocol::HandleGetAuthoringEntryListResult(data) => &data.request_id,
            Lib3hClientProtocol::HandleGetGossipingEntryListResult(data) => &data.request_id,
            Lib3hClientProtocol::PublishEntry(_) => "",
            Lib3hClientProtocol::HoldEntry(_) => "",
            _ => unimplemented!(),
        }
        .to_string();

        let result = if request_id == "" {
            self.engine.publish(Lib3hSpan::todo(), client_msg.into())
        } else {
            self.engine.request(
                Lib3hSpan::todo(),
                client_msg.into(),
                LegacyLib3h::make_callback(request_id.to_string()),
            )
        };
        result.map_err(|e| Lib3hProtocolError::new(ErrorKind::Other(e.to_string())))
    }

    /// Process Lib3hClientProtocol message inbox and
    /// output a list of Lib3hServerProtocol messages for Core to handle
    fn process(&mut self) -> Lib3hProtocolResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        detach_run!(&mut self.engine, |lib3h| lib3h.process(self))
            .map_err(|e| Lib3hProtocolError::new(ErrorKind::Other(e.to_string())))?;

        // get any "server" messages that came as responses to the client requests
        let mut responses: Vec<_> = self.client_request_responses.drain(0..).collect();

        for mut msg in self.engine.as_mut().drain_messages() {
            let server_msg = msg.take_message().expect("exists");
            responses.push(server_msg.into());
        }

        Ok((responses.len() > 0, responses))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib3h_protocol::data_types::*;
    use lib3h_tracing::test_span;
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
                ClientToLib3h::Connect(_data) => {
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

        /// create a fake lib3h event
        pub fn inject_lib3h_event(&mut self, msg: Lib3hToClient) {
            let _ = self
                .lib3h_endpoint
                .publish(test_span("inject_lib3h_event"), msg);
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
            peer_uri: Url::parse("mocknet://t1").expect("can parse url"),
            network_id: "fake_id".to_string(),
        };

        assert!(legacy.post(Lib3hClientProtocol::Connect(data)).is_ok());
        // process via the wrapper
        let result = legacy.process();

        // The mock engine allways returns failure on connect requests
        assert_eq!(
            "Ok((true, [FailureResult(GenericResultData { request_id: \"foo_request_id\", space_address: HashString(\"space_addr\"), to_agent_id: HashString(\"to_agent_id\"), result_info: \"connection failed!\" })]))",
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
            "Ok((true, [FailureResult(GenericResultData { request_id: \"bar_request_id\", space_address: HashString(\"space_addr\"), to_agent_id: HashString(\"to_agent_id\"), result_info: \"\" })]))",
            format!("{:?}", result)
        );

        detach_run!(&mut legacy.engine, |l| l.as_mut().inject_lib3h_event(
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

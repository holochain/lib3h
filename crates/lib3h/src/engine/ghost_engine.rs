use detach::Detach;
use lib3h_protocol::{
    data_types::GenericResultData, error::Lib3hProtocolResult, protocol::*, protocol_client::*,
    protocol_server::*, DidWork,
};

use lib3h_ghost_actor::*;

struct CoreContext(String);

type EngineError = String;

pub struct GhostEngine {
    endpoint_for_parent: Option<
        GhostEndpoint<
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hToClient,
            Lib3hToClientResponse,
            EngineError,
        >,
    >,
    endpoint_as_child: Detach<
        GhostContextEndpoint<
            GhostEngine,
            String,
            Lib3hToClient,
            Lib3hToClientResponse,
            ClientToLib3h,
            ClientToLib3hResponse,
            EngineError,
        >,
    >,
}

impl GhostEngine {
    pub fn new() -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        Self {
            endpoint_for_parent: Some(endpoint_parent),
            endpoint_as_child: Detach::new(
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
    > for GhostEngine
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
        std::mem::replace(&mut self.endpoint_for_parent, None)
    }
    // END BOILER PLATE--------------------------

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // START BOILER PLATE--------------------------
        // always run the endpoint process loop
        detach_run!(&mut self.endpoint_as_child, |cs| { cs.process(self) })?;
        // END BOILER PLATE--------------------------

        for msg in self.endpoint_as_child.as_mut().drain_messages() {
            self.handle_msg_from_node(msg)?;
        }

        Ok(true.into())
    }
}

impl GhostEngine {
    fn handle_msg_from_node(
        &mut self,
        mut msg: GhostMessage<ClientToLib3h, Lib3hToClient, ClientToLib3hResponse, EngineError>,
    ) -> Result<(), EngineError> {
        match msg.take_message().expect("exists") {
            ClientToLib3h::Connect(_data) => {
                // pretend the connection request failed
                msg.respond(Err("connection failed!".to_string()));
            }
            _ => panic!("{:?} not implemented", msg),
        }
        Ok(())
    }

    /// create a fake lib3h event
    pub fn inject_lib3h_event(&mut self, msg: Lib3hToClient) {
        self.endpoint_as_child.publish(msg);
    }
}

type GhostEngineParentWapper<Core> = GhostParentWrapper<
    Core,
    CoreContext,
    Lib3hToClient,
    Lib3hToClientResponse,
    ClientToLib3h,
    ClientToLib3hResponse,
    EngineError,
    GhostEngine,
>;

/// A wrapper for talking to lib3h using the legacy Lib3hClient/Server enums
#[allow(dead_code)]
struct LegacyLib3h {
    lib3h: Detach<GhostEngineParentWapper<LegacyLib3h>>,
    #[allow(dead_code)]
    name: String,
    client_request_responses: Vec<Lib3hServerProtocol>,
}

#[allow(dead_code)]
impl LegacyLib3h {
    pub fn new(name: &str) -> Self {
        LegacyLib3h {
            lib3h: Detach::new(GhostParentWrapper::new(GhostEngine::new(), name)),
            name: name.into(),
            client_request_responses: Vec::new(),
        }
    }

    fn server_failure(err: EngineError, context: CoreContext) -> Lib3hServerProtocol {
        let failure_data = GenericResultData {
            request_id: context.0,
            space_address: "space_addr".into(),
            to_agent_id: "to_agent_id".into(),
            result_info: err.as_bytes().to_vec(),
        };
        Lib3hServerProtocol::FailureResult(failure_data)
    }

    fn make_callback() -> GhostCallback<LegacyLib3h, CoreContext, ClientToLib3hResponse, EngineError>
    {
        Box::new(
            |me: &mut LegacyLib3h,
             context: CoreContext,
             response: GhostCallbackData<ClientToLib3hResponse, EngineError>| {
                match response {
                    GhostCallbackData::Response(Ok(rsp)) => {
                        me.client_request_responses.push(rsp.into());
                    }
                    GhostCallbackData::Response(Err(e)) => {
                        me.client_request_responses
                            .push(LegacyLib3h::server_failure(e, context));
                    }
                    GhostCallbackData::Timeout => {
                        me.client_request_responses
                            .push(LegacyLib3h::server_failure(
                                "Request timed out".into(),
                                context,
                            ));
                    }
                };
                Ok(())
            },
        )
    }

    /// Add incoming Lib3hClientProtocol message in FIFO
    fn post(&mut self, client_msg: Lib3hClientProtocol) -> Lib3hProtocolResult<()> {
        let ctx = match &client_msg {
            Lib3hClientProtocol::Connect(data) => CoreContext(data.request_id.clone()),
            Lib3hClientProtocol::JoinSpace(data) => CoreContext(data.request_id.clone()),
            Lib3hClientProtocol::LeaveSpace(data) => CoreContext(data.request_id.clone()),
            Lib3hClientProtocol::SendDirectMessage(data) => CoreContext(data.request_id.clone()),
            Lib3hClientProtocol::FetchEntry(data) => CoreContext(data.request_id.clone()),
            Lib3hClientProtocol::QueryEntry(data) => CoreContext(data.request_id.clone()),
            _ => panic!("unimplemented"),
        };
        self.lib3h
            .request(ctx, client_msg.into(), LegacyLib3h::make_callback());
        Ok(())
    }

    /// Process Lib3hClientProtocol message inbox and
    /// output a list of Lib3hServerProtocol messages for Core to handle
    fn process(&mut self) -> Lib3hProtocolResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        let _ = detach_run!(&mut self.lib3h, |lib3h| lib3h.process(self));

        // get any "server" messages that came as responses to the client requests
        let mut responses: Vec<_> = self.client_request_responses.drain(0..).collect();

        for mut msg in self.lib3h.as_mut().drain_messages() {
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
    struct MockCore {
        //    state: String,
    }
    use url::Url;

    #[test]
    fn test_ghost_engine() {
        let mut _core = MockCore {
    //        state: "".to_string(),
        };

        // create the legacy lib3h engine wrapper
        let mut legacy: LegacyLib3h = LegacyLib3h::new("core");

        let data = ConnectData {
            request_id: "foo_request_id".into(),
            peer_uri: Url::parse("mocknet://t1").expect("can parse url"),
            network_id: "fake_id".to_string(),
        };

        assert!(legacy.post(Lib3hClientProtocol::Connect(data)).is_ok());
        // process via the wrapper
        let result = legacy.process();

        assert_eq!(
            "Ok((true, [FailureResult(GenericResultData { request_id: \"foo_request_id\", space_address: HashString(\"space_addr\"), to_agent_id: HashString(\"to_agent_id\"), result_info: [99, 111, 110, 110, 101, 99, 116, 105, 111, 110, 32, 102, 97, 105, 108, 101, 100, 33] })]))",
            format!("{:?}", result)
        );

        detach_run!(&mut legacy.lib3h, |l| l.as_mut().inject_lib3h_event(
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

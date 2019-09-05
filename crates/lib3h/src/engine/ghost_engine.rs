use detach::Detach;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::protocol::*;
use crate::error::Lib3hError;

/// the context when making a request from core
/// this is always the request_id
pub struct CoreRequestContext(String);
impl CoreRequestContext {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
    pub fn get_request_id(&self) -> String {
        self.0.clone()
    }
}

/// this is a generic parent wrapper for a GhostEngine.  This allows us to have
/// a mock GhostEngine for proving out the LegacyLib3h wrapper
pub type GhostEngineParentWrapper<Core, Engine, EngineError> = GhostParentWrapper<
    Core,
    CoreRequestContext,
    Lib3hToClient,
    Lib3hToClientResponse,
    ClientToLib3h,
    ClientToLib3hResponse,
    EngineError,
    Engine,
>;

pub type ClientToLib3hMessage =
    GhostMessage<ClientToLib3h, Lib3hToClient, ClientToLib3hResponse, Lib3hError>;

pub type DhtToParentMessage =
    GhostMessage<Lib3hToClient, ClientToLib3h, Lib3hToClientResponse, Lib3hError>;

pub struct GhostEngine {
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
            GhostEngine,
            String,
            Lib3hToClient,
            Lib3hToClientResponse,
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hError,
        >,
    >,
}

impl GhostEngine {
    pub fn new(name: &str) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        Self {
            client_endpoint: Some(endpoint_parent),
            lib3h_endpoint: Detach::new(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix(name)
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
        Lib3hError,
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

impl GhostEngine {
    fn handle_msg_from_client(
        &mut self,
        mut msg: GhostMessage<ClientToLib3h, Lib3hToClient, ClientToLib3hResponse, Lib3hError>,
    ) -> Result<(), GhostError> {
        match msg.take_message().expect("exists") {
            ClientToLib3h::Connect(_data) => {
                // pretend the connection request failed
                msg.respond(Err(Lib3hError::new_other("connection failed!".into())));
            }
            ClientToLib3h::JoinSpace(_data) => {
                // pretend the request succeeded
                msg.respond(Ok(ClientToLib3hResponse::JoinSpaceResult));
            }
            _ => panic!("{:?} not implemented", msg),
        }
        Ok(())
    }

    /// create a fake lib3h event
    pub fn inject_lib3h_event(&mut self, msg: Lib3hToClient) {
        self.lib3h_endpoint.publish(msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //    use lib3h_protocol::data_types::*;
    struct MockCore {
        //    state: String,
    }
    //  use url::Url;

    #[test]
    fn test_ghost_engine() {
        let mut _core = MockCore {
            //        state: "".to_string(),
        };
        let _lib3h: GhostEngineParentWrapper<MockCore, GhostEngine, Lib3hError> =
            GhostParentWrapper::new(GhostEngine::new("test_engine"), "test_engine");
        assert!(true);
    }
}

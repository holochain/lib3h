use detach::Detach;
use lib3h_protocol::{
    protocol::*,
};

use lib3h_ghost_actor::*;

type EngineError = String;

pub struct GhostEngine {
    endpoint_for_parent: Option<
        GhostEndpoint<
            NodeToLib3h,
            NodeToLib3hResponse,
            Lib3hToNode,
            Lib3hToNodeResponse,
            EngineError,
        >,
    >,
    endpoint_as_child: Detach<
        GhostContextEndpoint<
            GhostEngine,
            String,
            Lib3hToNode,
            Lib3hToNodeResponse,
            NodeToLib3h,
            NodeToLib3hResponse,
            EngineError,
        >,
    >,
}

impl GhostEngine {
    pub fn new() -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        Self {
            endpoint_for_parent: Some(endpoint_parent),
            endpoint_as_child: Detach::new(endpoint_self.as_context_endpoint_builder().
                                           request_id_prefix("engine").build()),
        }
    }
}

impl GhostActor<Lib3hToNode, Lib3hToNodeResponse, NodeToLib3h, NodeToLib3hResponse, EngineError>
    for GhostEngine
{
    // START BOILER PLATE--------------------------
    fn take_parent_endpoint(
        &mut self,
    ) -> Option<
        GhostEndpoint<
            NodeToLib3h,
            NodeToLib3hResponse,
            Lib3hToNode,
            Lib3hToNodeResponse,
            EngineError,
        >,
    > {
        std::mem::replace(&mut self.endpoint_for_parent, None)
    }
    // END BOILER PLATE--------------------------

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // START BOILER PLATE--------------------------
        // always run the endpoint process loop
        detach_run!(&mut self.endpoint_as_child, |cs| {
            cs.process(self)
        })?;
        // END BOILER PLATE--------------------------

        // when processing just print all the messages
        self.endpoint_as_child
            .as_mut()
            .drain_messages()
            .iter_mut()
            .for_each(|msg| println!("{:?}", msg.take_message().unwrap()));

        Ok(true.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib3h_protocol::{
        data_types::*,
    };
    struct MockCore {
    //    state: String,
    }
    #[derive(Debug)]
    struct CoreContext(String);
    use url::Url;

    #[test]
    fn test_ghost_engine() {
        let mut core = MockCore {
    //        state: "".to_string(),
        };

        // create the wrapped lib3h engine
        let mut lib3h: GhostParentWrapper<
                MockCore,
            CoreContext,
            Lib3hToNode,
            Lib3hToNodeResponse,
            NodeToLib3h,
            NodeToLib3hResponse,
            EngineError,
            GhostEngine,
            > = GhostParentWrapper::new(GhostEngine::new(), "core");

        let data = ConnectData {
            request_id: "foo_request_id".into(),
            peer_uri: Url::parse("mocknet://t1").expect("can parse url"),
            network_id: "fake_id".to_string(),
        };

        lib3h.publish(NodeToLib3h::Connect(data));

        // process via the wrapper
        assert!(lib3h.process(&mut core).is_ok());

    }
}

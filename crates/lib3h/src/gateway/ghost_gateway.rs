use lib3h_ghost_actor::{
    prelude::*,
    ghost_channel::*,
};
use crate::transport::memory_mock::ghost_transport_memory::*;

/// Gateway Actor where:
///   - child: Transport
///   - parent: RealEngine or Multiplexer
pub struct GhostGateway<
    'gateway,
    D: Dht,
    RequestToParentContext,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    FakeError,
> {
    /// Used for distinguishing gateways
    identifier: String,
    /// Map holding the reversed mapping between connection url and connectionId response
    connection_map: HashMap<Url, ConnectionId>,
    /// Internal DHT
    inner_dht: D,
    /// Hold Endpoint to child actor
    // inner_transport: TransportWrapper<'gateway>,
    child_transport: Detach<
        GhostParentContextChannel<
            RequestToParentContext,
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            FakeError,
        >,
    >,
    /// Our channel to parent actor
    endpoint_for_parent: Option<GhostTransportMemoryChannel>,
    endpoint_from_parent: Option<GhostTransportMemoryChannelContext>,
}

impl<
    'gateway,
    D: Dht,
    RequestToParentContext,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    FakeError,
> GhostGateway<
    'gateway,
    D,
RequestToParentContext,
RequestToParent,
RequestToParentResponse,
RequestToChild,
RequestToChildResponse,
FakeError,
> {
    #[allow(dead_code)]
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new(
        identifier: &str,
        inner_transport: TransportWrapper<'gateway>,
        dht_factory: DhtFactory<D>,
        dht_config: &DhtConfig,
    ) -> Self {
        let (endpoint_for_parent, endpoint_from_parent) = create_ghost_channel();
        let child_transport = Detach::new(GhostParentContextChannel::new(
            Box::new(inner_transport),
            "to_child_transport",
        ));
        GhostGateway {
            endpoint_for_parent: Some(endpoint_for_parent),
            endpoint_from_parent: Some(endpoint_from_parent.as_context_channel("tmem_to_parent")),
            child_transport,
            inner_dht: dht_factory(dht_config).expect("Failed to construct DHT"),
            identifier: identifier.to_owned(),
            connection_map: HashMap::new(),
        }
    }
}

impl<
    'gateway,
    D: Dht,
    RequestToParentContext,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    FakeError,
> GhostActor<
        RequestToParentContext,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        FakeError,
    > for GhostGateway<
'gateway,
D,
RequestToParentContext,
RequestToParent,
RequestToParentResponse,
RequestToChild,
RequestToChildResponse,
FakeError,
> {

    // BOILERPLATE START ----------------------------------

    fn as_any(&mut self) -> &mut dyn Any {
        &mut *self
    }

    fn take_parent_channel(&mut self) -> Option<GhostChannel<
        RequestToChild,
        RequestToChildResponse,
        RequestToParent,
        RequestToParentResponse,
        FakeError,
    >,
    > {
        std::mem::replace(&mut self.endpoint_for_parent, None)
    }

    // BOILERPLATE END ----------------------------------

    /// Process
    fn /* priv */ process_concrete(&mut self) -> GhostResult<WorkWasDone> {

        // Process child
        detach_run!(&mut self.child_transport, |child_transport| { child_transport.process(self.as_any()) })?;

        // check inbox from parent
        detach_run!(&mut self.endpoint_from_parent, |endpoint_from_parent| {
                endpoint_from_parent.process(self.as_any())
            })?;
        for mut msg in self.channel_self.as_mut().drain_messages() {
            match msg.take_message().expect("exists") {
                _ => (),
            }
        }
        // process internal dht
        let did_some_work = self.inner_dht.process()?;

        Ok(did_some_work)
    }

}
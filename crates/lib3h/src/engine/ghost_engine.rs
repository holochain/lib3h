use detach::Detach;
use lib3h_ghost_actor::prelude::*;
use std::collections::{HashMap, HashSet};
use lib3h_protocol::{
    Address,
    data_types::*,
    protocol::*,
};

use super::RealEngineTrackerData;
use crate::{
    dht::{
        dht_trait::*,
    },
    engine::{
        //p2p_protocol::P2pProtocol,
        RealEngineConfig, TransportKeys, NETWORK_GATEWAY_ID,
        ChainId
    },
    gateway::{GatewayWrapper,P2pGateway},
    error::{Lib3hError,Lib3hResult,},
    track::Tracker,
    transport::{
        //protocol::TransportCommand,
        TransportWrapper, ConnectionId},
};
use lib3h_crypto_api::{CryptoSystem};
//use url::Url;
//use rmp_serde::{Deserializer, Serializer};
//use serde::{Deserialize, Serialize};


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

pub type Lib3hToClientMessage =
    GhostMessage<Lib3hToClient, ClientToLib3h, Lib3hToClientResponse, Lib3hError>;

pub type MockGateway = String;

#[allow(dead_code)]
pub struct GhostEngine<'engine, D: Dht+ 'engine > {
    /// Identifier
    name: String,
    /// Config settings
    config: RealEngineConfig,
    /// Factory for building DHT's of type D
    dht_factory: DhtFactory<D>,
    /// Tracking request_id's sent to core
    request_track: Tracker<RealEngineTrackerData>,
    // TODO #176: Remove this if we resolve #176 without it.
    #[allow(dead_code)]
    /// Transport used by the network gateway
    network_transport: TransportWrapper<'engine>,
    /// P2p gateway for the network layer
    network_gateway: GatewayWrapper<'engine>,
    /// Store active connections?
    network_connections: HashSet<ConnectionId>,
    /// Map of P2p gateway per Space+Agent
    space_gateway_map: HashMap<ChainId, MockGateway>,// GatewayWrapper<'engine>>,
    #[allow(dead_code)]
    /// crypto system to use
    crypto: Box<dyn CryptoSystem>,
    #[allow(dead_code)]
    /// transport_id data, public/private keys, etc
    transport_keys: TransportKeys,
    /// debug: count number of calls to process()
    process_count: u64,

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
            GhostEngine<'engine, D> ,
            String,
            Lib3hToClient,
            Lib3hToClientResponse,
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hError,
        >,
    >,
}

impl<'engine, D: Dht> GhostEngine<'engine, D> {
    pub fn new(
        name: &str,
        crypto: Box<dyn CryptoSystem>,
        config: RealEngineConfig,
        dht_factory: DhtFactory<D>,
        network_transport: TransportWrapper<'engine>,
    ) -> Lib3hResult<Self> {
        let binding = network_transport.as_mut().bind(&config.bind_url)?;
        // Generate keys
        // TODO #209 - Check persistence first before generating
        let transport_keys = TransportKeys::new(crypto.as_crypto_system())?;
        // Generate DHT config and create network_gateway
        let dht_config = DhtConfig {
            this_peer_address: transport_keys.transport_id.clone(),
            this_peer_uri: binding,
            custom: config.dht_custom_config.clone(),
            gossip_interval: config.dht_gossip_interval,
            timeout_threshold: config.dht_timeout_threshold,
        };
        let network_gateway = GatewayWrapper::new(P2pGateway::new(
            NETWORK_GATEWAY_ID,
            network_transport.clone(),
            dht_factory,
            &dht_config,
        ));
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        Ok(GhostEngine {
            crypto,
            config,
            name: name.to_string(),
            dht_factory,
            request_track: Tracker::new("real_engine_", 2000),
            network_transport,
            network_gateway,
            network_connections: HashSet::new(),
            space_gateway_map: HashMap::new(),
            transport_keys,
            process_count: 0,
            client_endpoint: Some(endpoint_parent),
            lib3h_endpoint: Detach::new(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix(name)
                    .build(),
            ),
        })
    }
}

impl<'engine, D: Dht>
    GhostActor<
        Lib3hToClient,
        Lib3hToClientResponse,
        ClientToLib3h,
        ClientToLib3hResponse,
        Lib3hError,
    > for GhostEngine<'engine, D>
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

/// Drop
impl<'engine, D: Dht> Drop for GhostEngine<'engine, D> {
    fn drop(&mut self) {
        self.shutdown().unwrap_or_else(|e| {
            warn!("Graceful shutdown failed: {}", e);
        });
    }
}

/// Private
impl<'engine, D: Dht> GhostEngine<'engine, D> {
    /// Called on drop.
    /// Close all connections gracefully
    fn shutdown(&mut self) -> Lib3hResult<()> {
        Ok(())
     /*   let mut result: Lib3hResult<()> = Ok(());

        for space_gatway in self.space_gateway_map.values_mut() {
            let res = space_gatway.as_transport_mut().close_all();
            // Continue closing connections even if some failed
            if let Err(e) = res {
                if result.is_ok() {
                    result = Err(e.into());
                }
            }
        }
        // Done
        self.network_gateway
            .as_transport_mut()
            .close_all()
            .map_err(|e| {
                error!("Closing of some connection failed: {:?}", e);
                e
            })?;

        result*/
    }

    /// Process any Client events or requests
    fn handle_msg_from_client(
        &mut self,
        mut msg: ClientToLib3hMessage,
    ) -> Result<(), GhostError> {
        match msg.take_message().expect("exists") {
            ClientToLib3h::Connect(_data) => {}
            //    let cmd = TransportCommand::Connect(data.peer_uri, data.request_id);
            //  self.network_gateway.as_transport_mut().post(cmd)?;
            // ARG need this to be a request with a callback!!
            // msg.respond(Err(Lib3hError::new_other("connection failed!".into())));

            ClientToLib3h::JoinSpace(data) => {
                let result = self.handle_join(&data).map(|_| ClientToLib3hResponse::JoinSpaceResult);
                msg.respond(result);
            }
/*            LeaveSpace(SpaceData) => {}
            SendDirectMessage(DirectMessageData) => {}
            FetchEntry(FetchEntryData)  => {}
            PublishEntry(ProvidedEntryData) => {}
            HoldEntry(ProvidedEntryData)  => {}
            QueryEntry(QueryEntryData) => {}*/

            _ => panic!("{:?} not implemented", msg),
        }
        Ok(())
    }

    fn add_gateway(&mut self, space_address: Address, agent_id: Address ) -> Lib3hResult<()> {
        let chain_id = (space_address, agent_id);
        if self.space_gateway_map.contains_key(&chain_id) {
            return Err(Lib3hError::new_other("Already joined space"));
        }
        self.space_gateway_map
            .insert(chain_id.clone(), format!("Gateway for: {:?}",chain_id));
        Ok(())
    }

    /// Create a gateway for this agent in this space, if not already part of it.
    /// Must not already be part of this space.
    fn handle_join(&mut self, join_msg: &SpaceData) -> Lib3hResult<()> {
        self.add_gateway(join_msg.space_address.clone(), join_msg.agent_id.clone())?;
 /*       // Bail if space already joined by agent
        let chain_id = (join_msg.space_address.clone(), join_msg.agent_id.clone());
        if self.space_gateway_map.contains_key(&chain_id) {
            return Err(Lib3hError::new_other("Already joined space"));
        }

        // First create DhtConfig for space gateway
        let agent_id: String = join_msg.agent_id.clone().into();
        let this_peer_transport_id_as_uri = {
            let gateway = self.network_gateway.as_ref();
            // TODO #175 - encapsulate this conversion logic
            Url::parse(format!("transportId:{}", gateway.this_peer().peer_address).as_str())
                .expect("can parse url")
        };
        let dht_config = DhtConfig {
            this_peer_address: agent_id,
            this_peer_uri: this_peer_transport_id_as_uri,
            custom: self.config.dht_custom_config.clone(),
            gossip_interval: self.config.dht_gossip_interval,
            timeout_threshold: self.config.dht_timeout_threshold,
        };
        // Create new space gateway for this ChainId
        let new_space_gateway: GatewayWrapper<'engine> =
            GatewayWrapper::new(P2pGateway::new_with_space(
                self.network_gateway.as_transport(),
                &join_msg.space_address,
                self.dht_factory,
                &dht_config,
            ));

        // TODO #150 - Send JoinSpace to all known peers
        let space_address: String = join_msg.space_address.clone().into();
        let peer = new_space_gateway.as_ref().this_peer().to_owned();
        let mut payload = Vec::new();
        let p2p_msg = P2pProtocol::BroadcastJoinSpace(space_address.clone(), peer.clone());
        p2p_msg
            .serialize(&mut Serializer::new(&mut payload))
            .unwrap();
        trace!(
            "{} - Broadcasting JoinSpace: {}, {}",
            self.name,
            space_address,
            peer.peer_address,
        );
        self.network_gateway
            .as_transport_mut()
            .send_all(&payload)
            .ok();
        // TODO END

        // Add it to space map
        self.space_gateway_map
            .insert(chain_id.clone(), new_space_gateway);
        // Have DHT broadcast our PeerData
        let space_gateway = self.space_gateway_map.get_mut(&chain_id).unwrap();
        let this_peer = { space_gateway.as_ref().this_peer().clone() };
        space_gateway
            .as_dht_mut()
            .post(DhtCommand::HoldPeer(this_peer))?;
        // Send Get*Lists requests
        let mut list_data = GetListData {
            space_address: join_msg.space_address.clone(),
            provider_agent_id: join_msg.agent_id.clone(),
            request_id: self.request_track.reserve(),
        };
        self.request_track.set(
            &list_data.request_id,
            Some(RealEngineTrackerData::GetGossipingEntryList),
        );
        self.lib3h_endpoint.request(Lib3hToClient::HandleGetGossipingEntryList(
            list_data.clone(),
        ));
        list_data.request_id = self.request_track.reserve();
        self.request_track.set(
            &list_data.request_id,
            Some(RealEngineTrackerData::GetAuthoringEntryList),
        );
        output.push(Lib3hServerProtocol::HandleGetAuthoringEntryList(list_data));
*/
        // Done
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //    use lib3h_protocol::data_types::*;
    struct MockCore {
        //    state: String,
    }
    use url::Url;
    use crate::dht::mirror_dht::MirrorDht;
    use crate::transport::memory_mock::transport_memory::TransportMemory;
    use crate::transport_wss::TlsConfig;

    use lib3h_sodium::SodiumCryptoSystem;

    #[test]
    fn test_ghost_engine() {
        let mut _core = MockCore {
            //        state: "".to_string(),
        };

        let network_transport = TransportWrapper::new(TransportMemory::new());
        let crypto =  Box::new(SodiumCryptoSystem::new());
        let config = RealEngineConfig {
            tls_config: TlsConfig::Unencrypted,
            socket_type: "mem".into(),
            bootstrap_nodes: vec![],
            work_dir: String::new(),
            log_level: 'd',
            bind_url: Url::parse(format!("mem://{}", "test_engine").as_str()).unwrap(),
            dht_gossip_interval: 100,
            dht_timeout_threshold: 1000,
            dht_custom_config: vec![],
        };
        let dht_factory = MirrorDht::new_with_config;

        let engine : GhostEngine<MirrorDht> = GhostEngine::new(
            "test_engine",
            crypto,
            config,
            dht_factory,
            network_transport,
        ).unwrap();
        let mut lib3h: GhostEngineParentWrapper<MockCore, GhostEngine<MirrorDht>, Lib3hError> =
            GhostParentWrapper::new(engine, "test_engine");
        assert_eq!(lib3h.as_ref().space_gateway_map.len(),0);

        let req_data = SpaceData {
            /// Identifier of this request
            request_id: "foo_id".into(),
            space_address: "space_addr".into(),
            agent_id: "agent_id".into(),
        };

        let result = lib3h.as_mut().handle_join(&req_data);
        assert!(result.is_ok());
        assert_eq!(lib3h.as_ref().space_gateway_map.len(),1);
        let result = lib3h.as_mut().handle_join(&req_data);
        assert_eq!(
            "Err(Lib3hError(Other(\"Already joined space\")))",
            format!("{:?}", result)
        );

    }
}

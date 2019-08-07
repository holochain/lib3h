pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;

use crate::{
    dht::dht_trait::Dht,
    transport::{
        protocol::TransportCommand, transport_trait::Transport, ConnectionId, TransportWrapper,
    },
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use url::Url;

pub trait Gateway: Transport + Dht {
    fn identifier(&self) -> &str;
    fn get_connection_id(&self, peer_address: &str) -> Option<String>;
}

/// since rust doesn't suport upcasting to supertraits
/// create a super-fat-pointer in this wrapper struct
#[derive(Clone)]
pub struct GatewayWrapper<'wrap> {
    gateway: Arc<RwLock<dyn Gateway + 'wrap>>,
    transport: TransportWrapper<'wrap>,
    dht: Arc<RwLock<dyn Dht + 'wrap>>,
}

impl<'wrap> GatewayWrapper<'wrap> {
    pub fn new<T: Gateway + 'wrap>(concrete: &Arc<RwLock<T>>) -> Self {
        Self {
            gateway: concrete.clone(),
            transport: TransportWrapper::assume(concrete.clone()),
            dht: concrete.clone(),
        }
    }

    pub fn as_transport(&self) -> TransportWrapper<'wrap> {
        self.transport.clone()
    }

    pub fn as_transport_ref(&self) -> RwLockReadGuard<'_, dyn Transport + 'wrap> {
        self.transport.as_ref()
    }

    pub fn as_transport_mut(&self) -> RwLockWriteGuard<'_, dyn Transport + 'wrap> {
        self.transport.as_mut()
    }

    pub fn as_dht(&self) -> Arc<RwLock<dyn Dht + 'wrap>> {
        self.dht.clone()
    }

    pub fn as_dht_ref(&self) -> RwLockReadGuard<'_, dyn Dht + 'wrap> {
        self.dht.read().expect("can access")
    }

    pub fn as_dht_mut(&self) -> RwLockWriteGuard<'_, dyn Dht + 'wrap> {
        self.dht.write().expect("can access")
    }

    pub fn as_gateway(&self) -> Arc<RwLock<dyn Gateway + 'wrap>> {
        self.gateway.clone()
    }

    pub fn as_ref(&self) -> RwLockReadGuard<'_, dyn Gateway + 'wrap> {
        self.gateway.read().expect("can access")
    }

    pub fn as_mut(&self) -> RwLockWriteGuard<'_, dyn Gateway + 'wrap> {
        self.gateway.write().expect("can access")
    }
}

/// Gateway to a P2P network.
/// Combines a transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
/// P2pGateway should not `post() & process()` its inner transport but call it synchrounously.
pub struct P2pGateway<'gateway, D: Dht> {
    inner_transport: TransportWrapper<'gateway>,
    inner_dht: D,
    /// Used for distinguishing gateways
    identifier: String,
    /// Map holding the reversed mapping between connection url and connectionId response
    connection_map: HashMap<Url, ConnectionId>,
    /// Own inbox for TransportCommands which is processed during Transport::process()
    transport_inbox: VecDeque<TransportCommand>,
}

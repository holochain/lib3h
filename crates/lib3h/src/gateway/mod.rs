pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;

use crate::{
    dht::dht_trait::Dht,
    transport::{protocol::*, transport_trait::Transport, TransportWrapper},
};
use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use lib3h_protocol::Address;
use url::Url;

/// describes a super construct of a Transport and a Dht allowing
/// Transport access via peer discovery handled by the Dht
pub trait Gateway: Transport + Dht {
    fn identifier(&self) -> &str;
    fn inject_event(&mut self, evt: TransportEvent);
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
    /// create a super-fat trait-object pointer to access concrete gateway
    /// as a gateway, transport, or dht
    pub fn new<T: Gateway + 'wrap>(concrete: T) -> Self {
        let concrete = Arc::new(RwLock::new(concrete));
        Self {
            gateway: concrete.clone(),
            transport: TransportWrapper::assume(concrete.clone()),
            dht: concrete.clone(),
        }
    }

    /// clone a pointer to the internal TransportWrapper
    pub fn as_transport(&self) -> TransportWrapper<'wrap> {
        self.transport.clone()
    }

    /// immutable ref to the dyn Transport
    pub fn as_transport_ref(&self) -> RwLockReadGuard<'_, dyn Transport + 'wrap> {
        self.transport.as_ref()
    }

    /// mutable ref to the dyn Transport
    pub fn as_transport_mut(&self) -> RwLockWriteGuard<'_, dyn Transport + 'wrap> {
        self.transport.as_mut()
    }

    /// clone a pointer to the internal dyn Dht
    pub fn as_dht(&self) -> Arc<RwLock<dyn Dht + 'wrap>> {
        self.dht.clone()
    }

    /// immutable ref to the dyn Dht
    pub fn as_dht_ref(&self) -> RwLockReadGuard<'_, dyn Dht + 'wrap> {
        self.dht.read().expect("failed to obtain read lock")
    }

    /// mutable ref to the dyn Dht
    pub fn as_dht_mut(&self) -> RwLockWriteGuard<'_, dyn Dht + 'wrap> {
        self.dht.write().expect("failed to obtain write lock")
    }

    /// clone a pointer to the internal dyn Gateway
    pub fn as_gateway(&self) -> Arc<RwLock<dyn Gateway + 'wrap>> {
        self.gateway.clone()
    }

    /// immutable ref to the dyn Gateway
    pub fn as_ref(&self) -> RwLockReadGuard<'_, dyn Gateway + 'wrap> {
        self.gateway.read().expect("failed to obtain read lock")
    }

    /// mutable ref to the dyn Gateway
    pub fn as_mut(&self) -> RwLockWriteGuard<'_, dyn Gateway + 'wrap> {
        self.gateway.write().expect("failed to obtain write lock")
    }
}

/// Gateway to a P2P network.
/// Combines a transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
/// P2pGateway should not `post() & process()` its inner transport but call it synchrounously.
#[allow(dead_code)]
pub struct P2pGateway<'gateway, D: Dht> {
    address_url_scheme: String,
    space_address: Address,
    //inner_transport: TransportWrapper<'gateway>,
    inner_dht: D,
    /// Used for distinguishing gateways
    identifier: String,
    /// Map holding the open connections we have
    connections: HashSet<Url>,
    /// Own inbox for TransportCommands which is processed during Transport::process()
    transport_inbox: VecDeque<TransportCommand>,
    transport_injected_events: Vec<TransportEvent>,
    transport_sends: Vec<(String, Url, Vec<u8>)>,
    phantom_data: std::marker::PhantomData<&'gateway i8>,
}

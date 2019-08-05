pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;

use crate::{
    dht::dht_trait::Dht,
    transport::{protocol::TransportCommand, transport_trait::Transport, ConnectionId},
};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use url::Url;

pub trait Gateway: Transport + Dht {
    fn identifier(&self) -> &str;
    fn get_connection_id(&self, peer_address: &str) -> Option<String>;
}

/// since rust doesn't suport upcasting to supertraits
/// create a super-fat-pointer in this wrapper struct
#[derive(Clone)]
pub struct GatewayWrapper {
    gateway: Rc<RefCell<dyn Gateway>>,
    transport: Rc<RefCell<dyn Transport>>,
    dht: Rc<RefCell<dyn Dht>>,
}

impl GatewayWrapper {
    pub fn new<T: Gateway + 'static>(concrete: &Rc<RefCell<T>>) -> Self {
        Self {
            gateway: concrete.clone(),
            transport: concrete.clone(),
            dht: concrete.clone(),
        }
    }

    pub fn as_transport(&self) -> Rc<RefCell<dyn Transport>> {
        self.transport.clone()
    }

    pub fn as_transport_ref(&self) -> TransportRef {
        TransportRef(self.transport.borrow())
    }

    pub fn as_transport_mut(&self) -> TransportRefMut {
        TransportRefMut(self.transport.borrow_mut())
    }

    pub fn as_dht(&self) -> Rc<RefCell<dyn Dht>> {
        self.dht.clone()
    }

    pub fn as_dht_ref(&self) -> DhtRef {
        DhtRef(self.dht.borrow())
    }

    pub fn as_dht_mut(&self) -> DhtRefMut {
        DhtRefMut(self.dht.borrow_mut())
    }

    pub fn as_gateway(&self) -> Rc<RefCell<dyn Gateway>> {
        self.gateway.clone()
    }

    pub fn as_ref(&self) -> GatewayRef {
        GatewayRef(self.gateway.borrow())
    }

    pub fn as_mut(&self) -> GatewayRefMut {
        GatewayRefMut(self.gateway.borrow_mut())
    }
}

pub struct TransportRef<'a>(std::cell::Ref<'a, dyn Transport>);

impl<'a> std::ops::Deref for TransportRef<'a> {
    type Target = dyn Transport + 'a;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

pub struct TransportRefMut<'a>(std::cell::RefMut<'a, dyn Transport>);

impl<'a> std::ops::Deref for TransportRefMut<'a> {
    type Target = dyn Transport + 'a;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl std::ops::DerefMut for TransportRefMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

pub struct DhtRef<'a>(std::cell::Ref<'a, dyn Dht>);

impl<'a> std::ops::Deref for DhtRef<'a> {
    type Target = dyn Dht + 'a;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

pub struct DhtRefMut<'a>(std::cell::RefMut<'a, dyn Dht>);

impl<'a> std::ops::Deref for DhtRefMut<'a> {
    type Target = dyn Dht + 'a;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl std::ops::DerefMut for DhtRefMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

pub struct GatewayRef<'a>(std::cell::Ref<'a, dyn Gateway>);

impl<'a> std::ops::Deref for GatewayRef<'a> {
    type Target = dyn Gateway + 'a;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

pub struct GatewayRefMut<'a>(std::cell::RefMut<'a, dyn Gateway>);

impl<'a> std::ops::Deref for GatewayRefMut<'a> {
    type Target = dyn Gateway + 'a;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl std::ops::DerefMut for GatewayRefMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

/// Gateway to a P2P network.
/// Combines a transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
/// P2pGateway should not `post() & process()` its inner transport but call it synchrounously.
pub struct P2pGateway<D: Dht> {
    inner_transport: Rc<RefCell<dyn Transport>>,
    inner_dht: D,
    /// Used for distinguishing gateways
    identifier: String,
    /// Map holding the reversed mapping between connection url and connectionId response
    connection_map: HashMap<Url, ConnectionId>,
    /// Own inbox for TransportCommands which is processed during Transport::process()
    transport_inbox: VecDeque<TransportCommand>,
}

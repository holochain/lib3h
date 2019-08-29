use crate::{
    dht::dht_trait::Dht,
    transport::{
        GhostTransportWrapper,
        protocol::TransportCommand, transport_trait::Transport, ConnectionId,
    },
    ghost_gateway::GhostGateway,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use url::Url;

/// since rust doesn't suport upcasting to supertraits
/// create a super-fat-pointer in this wrapper struct
#[derive(Clone)]
pub struct GhostGatewayWrapper<D: Dht> {
    inner: Arc<RwLock<GhostGateway<D>>>,
}

impl<D: Dht> GhostGatewayWrapper<D> {
    /// create a super-fat trait-object pointer to access concrete gateway
    /// as a gateway, transport, or dht
    pub fn new<T: Dht>(concrete_gateway: GhostGateway<D>) -> Self {
        let concrete_gateway = Arc::new(RwLock::new(concrete_gateway));
        Self {
            inner: concrete_gateway.clone(),
        }
    }

    /// immutable ref to the dyn Gateway
    pub fn as_ref(&self) -> RwLockReadGuard<GhostGateway<D>> {
        self.inner.read().expect("failed to obtain read lock")
    }

    /// mutable ref to the dyn Gateway
    pub fn as_mut(&self) -> RwLockWriteGuard<GhostGateway<D>> {
        self.inner.write().expect("failed to obtain write lock")
    }

/*
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
    pub fn as_ghost_gateway(&self) -> Arc<RwLock<dyn Gateway + 'wrap>> {
        self.gateway.clone()
    }
*/
}

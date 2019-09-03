use crate::{
    gateway::Gateway,
    transport::{transport_trait::Transport, TransportWrapper},
};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// since rust doesn't suport upcasting to supertraits
/// create a super-fat-pointer in this wrapper struct
#[derive(Clone)]
pub struct GatewayWrapper<'wrap> {
    gateway: Arc<RwLock<dyn Gateway + 'wrap>>,
    transport: TransportWrapper<'wrap>,
}

impl<'wrap> GatewayWrapper<'wrap> {
    /// create a super-fat trait-object pointer to access concrete gateway
    /// as a gateway, transport, or dht
    pub fn new<T: Gateway + 'wrap>(concrete: T) -> Self {
        let concrete = Arc::new(RwLock::new(concrete));
        Self {
            gateway: concrete.clone(),
            transport: TransportWrapper::assume(concrete.clone()),
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

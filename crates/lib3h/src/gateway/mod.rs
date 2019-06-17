pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht, rrdht::RrDht},
    engine::p2p_protocol::P2pProtocol,
    transport::{
        error::{TransportError, TransportResult},
        memory_mock::transport_memory::TransportMemory,
        protocol::{TransportCommand, TransportEvent},
        transport_trait::Transport,
        TransportId, TransportIdRef,
    },
    transport_wss::TransportWss,
};
use lib3h_protocol::{data_types::EntryData, AddressRef, DidWork, Lib3hResult};



/// Gateway to a P2P network.
/// Enables Connections to many other nodes.
/// Tracks distributed data for that P2P network.
/// P2pGateway should not `post() & process()` its inner transport but call it synchrounously.
/// Composite pattern for Transport and Dht
pub struct P2pGateway<'t, T: Transport, D: Dht> {
    inner_transport: &'t T,
    inner_dht: D,
    ///
    maybe_advertise: Option<String>,
}

use holochain_lib3h_protocol::{Lib3hResult, Address, DidWork};
use crate::dht::{
    dht_event::{
        DhtEvent, PeerHoldRequestData,
    }
};

pub trait Dht {
    fn init(&self, backend: String/*FIXME*/) -> Lib3hResult<()>;
    fn this_peer(&self) -> Lib3hResult<()>;

    fn get_peer(&self, peer_address: String) -> Option<PeerHoldRequestData>;
    fn fetch_peer(&self, peer_address: String) -> Option<PeerHoldRequestData>;
    fn drop_peer(&self, peer_address: String) -> Lib3hResult<()>;
    fn get_data(&self, data_address: Address) -> Lib3hResult<Vec<u8>>;
    fn fetch_data(&self, data_address: Address) -> Lib3hResult<Vec<u8>>;

    fn post(&mut self, evt: DhtEvent) -> Lib3hResult<()>;
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)>;
}

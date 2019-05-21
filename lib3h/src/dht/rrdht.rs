use holochain_lib3h_protocol::{Lib3hResult, Address, DidWork};

use crate::dht::{
    dht_trait::Dht,
    dht_event::{
        DhtEvent, PeerHoldRequestData,
    }
};

pub struct rrdht {}

impl rrdht {
    pub fn new() -> Self {
        rrdht {}
    }
}

impl Dht for rrdht {
    fn init(&self, backend: String/*FIXME*/) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }

    fn this_peer(&self) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }

    fn get_peer(&self, peer_address: String) -> Option<PeerHoldRequestData> {
        // FIXME
        None
    }
    fn fetch_peer(&self, peer_address: String) -> Option<PeerHoldRequestData> {
        // FIXME
        None
    }
    fn drop_peer(&self, peer_address: String) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn get_data(&self, data_address: Address) -> Lib3hResult<Vec<u8>> {
        // FIXME
        Ok(vec![])
    }
    fn fetch_data(&self, data_address: Address) -> Lib3hResult<Vec<u8>> {
        // FIXME
        Ok(vec![])
    }

    fn post(&mut self, evt: DhtEvent) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)> {
        // FIXME
        let mut outbox = Vec::new();
        let mut did_work = false;
        Ok((did_work, outbox))
    }
}

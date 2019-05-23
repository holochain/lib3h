use crate::dht::{
    dht_event::{DhtEvent, PeerHoldRequestData},
    dht_trait::Dht,
};
use holochain_lib3h_protocol::{Address, DidWork, Lib3hResult};
use std::collections::VecDeque;

/// RoundAndRound DHT implementation
pub struct RrDht {
    /// FIFO of DhtEvents send to us
    inbox: VecDeque<DhtEvent>,
}

impl RrDht {
    pub fn new() -> Self {
        RrDht {
            inbox: VecDeque::new(),
        }
    }
}

impl Dht for RrDht {
    // -- Getters -- //

    fn this_peer(&self) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }

    // -- Peer -- //

    fn get_peer(&self, _peer_address: String) -> Option<PeerHoldRequestData> {
        // FIXME
        None
    }
    fn fetch_peer(&self, _peer_address: String) -> Option<PeerHoldRequestData> {
        // FIXME
        None
    }
    fn drop_peer(&self, _peer_address: String) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }

    // -- Data -- //

    fn get_data(&self, _data_address: Address) -> Lib3hResult<Vec<u8>> {
        // FIXME
        Ok(vec![])
    }
    fn fetch_data(&self, _data_address: Address) -> Lib3hResult<Vec<u8>> {
        // FIXME
        Ok(vec![])
    }

    // -- Processing -- //

    fn post(&mut self, _evt: DhtEvent) -> Lib3hResult<()> {
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

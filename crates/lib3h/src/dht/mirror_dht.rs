use crate::dht::{
    dht_event::{DhtEvent, PeerHoldRequestData},
    dht_trait::Dht,
};
use lib3h_protocol::{AddressRef, DidWork, Lib3hResult};
use std::collections::VecDeque;

/// Mirror DHT implementation: Reflect everything back
pub struct MirrorDht {
    /// FIFO of DhtEvents send to us
    inbox: VecDeque<DhtEvent>,
    /// Storage of PeerHoldRequestData
    peer_list: HashMap<String, PeerHoldRequestData>,


}

impl RrDht {
    pub fn new() -> Self {
        RrDht {
            inbox: VecDeque::new(),
        }
    }
}

impl Dht for MirrorDht {
    // -- Getters -- //

    fn this_peer(&self) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }

    // -- Peer -- //

    fn get_peer(&self, _peer_address: &str) -> Option<PeerHoldRequestData> {
        // FIXME
        None
    }
    fn fetch_peer(&self, _peer_address: &str) -> Option<PeerHoldRequestData> {
        // FIXME
        None
    }
    fn drop_peer(&self, _peer_address: &str) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn get_peer_list(&self) -> Vec<PeerHoldRequestData> {
        // FIXME
        vec![]
    }

    // -- Data -- //

    fn get_data(&self, _data_address: &AddressRef) -> Lib3hResult<Vec<u8>> {
        // FIXME
        Ok(vec![])
    }
    fn fetch_data(&self, _data_address: &AddressRef) -> Lib3hResult<Vec<u8>> {
        // FIXME
        Ok(vec![])
    }

    // -- Processing -- //

    fn post(&mut self, evt: DhtEvent) -> Lib3hResult<()> {
        self.inbox.push_back(evt);
        Ok(())
    }

    // FIXME
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)> {
        let outbox = Vec::new();
        let mut did_work = false;
        loop {
            let evt = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            did_work = true;
            println!("(log.t) MirrorDht.process(): {:?}", evt)
        }
        Ok((did_work, outbox))
    }
}

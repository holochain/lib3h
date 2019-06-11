use crate::dht::{
    dht_event::{DhtEvent, PeerHoldRequestData},
    dht_trait::Dht,
};
use lib3h_protocol::{AddressRef, DidWork, Lib3hResult, data_types::EntryData};
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

    fn this_peer(&self) -> Lib3hResult<&str> {
        // FIXME
        Ok("FIXME")
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

    fn get_entry(&self, _data_address: &AddressRef) -> Option<EntryData> {
        // FIXME
        None
    }
    fn fetch_entry(&self, _data_address: &AddressRef) -> Option<EntryData> {
        // FIXME
        None
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
            println!("(log.t) RrDht.process(): {:?}", evt)
        }
        Ok((did_work, outbox))
    }
}

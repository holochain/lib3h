use crate::dht::{dht_protocol::*, dht_trait::Dht};
use lib3h_protocol::{data_types::EntryData, AddressRef, DidWork, Lib3hResult};
use std::collections::VecDeque;

/// RedRibbon DHT implementation
pub struct RrDht {
    /// FIFO of DhtCommands send to us
    inbox: VecDeque<DhtCommand>,
    ///
    this_peer: PeerData,
}

impl RrDht {
    pub fn new() -> Self {
        RrDht {
            inbox: VecDeque::new(),
            this_peer: PeerData {
                peer_address: "FIXME".to_string(),
                transport: "FIXME".to_string(),
                timestamp: 0, // FIXME
            },
        }
    }

    pub fn new_with_raw_config(_config: &[u8]) -> Lib3hResult<Self> {
        Ok(Self::new())
    }
}

impl Dht for RrDht {
    // -- Getters -- //

    fn this_peer(&self) -> &PeerData {
        &self.this_peer
    }

    // -- Peer -- //

    fn get_peer(&self, _peer_address: &str) -> Option<PeerData> {
        // FIXME
        None
    }
    fn fetch_peer(&self, _peer_address: &str) -> Option<PeerData> {
        // FIXME
        None
    }
    fn get_peer_list(&self) -> Vec<PeerData> {
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

    fn post(&mut self, cmd: DhtCommand) -> Lib3hResult<()> {
        self.inbox.push_back(cmd);
        Ok(())
    }

    // FIXME
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)> {
        let outbox = Vec::new();
        let mut did_work = false;
        loop {
            let cmd = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            did_work = true;
            println!("[t] RrDht.process(): {:?}", cmd)
        }
        Ok((did_work, outbox))
    }
}

use crate::dht::{dht_protocol::*, dht_trait::Dht};
use lib3h_protocol::{Address, DidWork, Lib3hResult};
use std::collections::VecDeque;
use url::Url;

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
                peer_uri: Url::parse("fixme://host:123").expect("a valid transport url"),
                timestamp: 0, // FIXME
            },
        }
    }

    pub fn new_with_raw_config(_config: &[u8]) -> Lib3hResult<Self> {
        Ok(Self::new())
    }
}

impl Dht for RrDht {
    // -- Peer info -- //

    fn get_peer_list(&self) -> Vec<PeerData> {
        // FIXME
        vec![]
    }

    fn get_peer(&self, _peer_address: &str) -> Option<PeerData> {
        // FIXME
        None
    }

    fn this_peer(&self) -> &PeerData {
        &self.this_peer
    }

    // -- Entry -- //

    fn get_entry_address_list(&self) -> Vec<&Address> {
        // FIXME
        vec![]
    }
    fn get_aspects_of(&self, _entry_address: &Address) -> Option<Vec<Address>> {
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
            trace!("RrDht.process(): {:?}", cmd)
        }
        Ok((did_work, outbox))
    }
}

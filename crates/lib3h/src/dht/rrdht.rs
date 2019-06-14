use crate::dht::{dht_protocol::*, dht_trait::Dht};
use lib3h_protocol::{data_types::EntryData, AddressRef, DidWork, Lib3hResult};
use std::collections::VecDeque;

/// RedRibbon DHT implementation
pub struct RrDht {
    /// FIFO of DhtCommands send to us
    inbox: VecDeque<DhtCommand>,
}

impl RrDht {
    pub fn new() -> RrDht {
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
            println!("(log.t) RrDht.process(): {:?}", cmd)
        }
        Ok((did_work, outbox))
    }
}

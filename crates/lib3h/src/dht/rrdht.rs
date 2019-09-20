use crate::{
    dht::{dht_protocol::*, dht_config::DhtConfig, Lib3hUri},
    error::Lib3hResult,
};
use lib3h_protocol::{Address, DidWork};
use std::collections::VecDeque;

/// RedRibbon DHT implementation
/// TODO #167
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
                peer_name: Lib3hUri::with_undefined("FIXME"),
                peer_location: Lib3hUri::with_undefined("host:123"),
                timestamp: 0, // TODO #166
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

    fn get_peer(&self, _peer_name: &Lib3hUri) -> Option<PeerData> {
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
            // FIXME
        }
        Ok((did_work, outbox))
    }
}

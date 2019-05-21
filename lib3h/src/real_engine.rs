use std::collections::{VecDeque, HashMap};

use holochain_lib3h_protocol::{
    network_engine::NetworkEngine,
    DidWork, Address,
    protocol::Lib3hProtocol,
    Lib3hResult,
};

use crate::{
    dht::{
        dht_trait::Dht,
        rrdht::rrdht,
        dht_event::DhtEvent,
    },
    p2p::{
        p2p_trait::P2p,
        p2p_event::P2pEvent,
        p2p_hackmode::P2pHackmode,
    },
};

/// Struct holding all config settings for the RealEngine
#[derive(Debug, Clone, PartialEq)]
pub struct RealEngineConfig {
    pub socket_type: String,
    pub bootstrap_nodes: Vec<String>,
    pub work_dir: String,
    pub log_level: char,
}

/// Lib3h's 'real mode' as a NetworkEngine
pub struct RealEngine {
    /// Config settings
    config: RealEngineConfig,
    /// FIFO of messages received from Core
    inbox: VecDeque<Lib3hProtocol>,
    /// Identifier
    name: String,
    // FIXME,
    transport_p2p: Box<P2p>,
    dna_dht_map: HashMap<Address, Box<P2p>>,
}

impl RealEngine {
    /// Constructor
    pub fn new(config: RealEngineConfig, name: &str) -> Lib3hResult<Self> {
        Ok(RealEngine {
            config,
            inbox: VecDeque::new(),
            name: name.to_string(),
            transport_p2p: Box::new(P2pHackmode::new()),
            dna_dht_map: HashMap::new(),
        })
    }
}

impl NetworkEngine for RealEngine {
    fn run(&self) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn stop(&self) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn terminate(&self) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn advertise(&self) -> String {
        "FIXME".to_string()
    }

    /// Add incoming message in FIFO
    fn post(&mut self, local_msg: Lib3hProtocol) -> Lib3hResult<()> {
        self.inbox.push_back(local_msg);
        Ok(())
    }

    /// Process FIFO and output a list of protocol messages for Core to handle
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<Lib3hProtocol>)> {
        // Process the transport layer
        let (did_work, p2p_event_list) = self.transport_p2p.process()?;
        if did_work {
            for evt in p2p_event_list {
                let protocol_output = self.serve_P2pEvent(evt)?;
                // Add protocol messages output to self's inbox
                for msg in protocol_output {
                    self.post(msg)?;
                }
            }
        }
        // Process all dna dhts
        for (dna, mut dht_p2p) in self.dna_dht_map {
            let (did_work, p2p_event_list) = dht_p2p.process()?;
            if did_work {
                for evt in p2p_event_list {
                    let protocol_output = self.serve_P2pEvent(evt)?;
                    // Add protocol messages output to self's inbox
                    for msg in protocol_output {
                        self.post(msg)?;
                    }
                }
            }
        }
        // Process all received Lib3hProtocol messages
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Progressively serve every protocol message in inbox
        loop {
            let local_msg = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            let (success, mut output) = self.serve_Lib3hProtocol(local_msg)?;
            if success {
                did_work = success;
            }
            outbox.append(&mut output);
        }
        // Done
        Ok((did_work, outbox))
    }
}

impl RealEngine {
    /// Process a Lib3hProtocol message sent to us (by Core)
    /// Return a list of Lib3hProtocol messages to send to others?
    fn serve_Lib3hProtocol(&self, local_msg: Lib3hProtocol) -> Lib3hResult<(DidWork, Vec<Lib3hProtocol>)> {
        println!("(log.d) >>>> '{}' recv: {:?}", self.name.clone(), local_msg);
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Note: use same order as the enum
        match local_msg {
            Lib3hProtocol::SuccessResult(_msg) => {
                // FIXME
            }
            Lib3hProtocol::FailureResult(_msg) => {
                // FIXME
            }
            Lib3hProtocol::TrackDna(_msg) => {
                // FIXME
            }
            Lib3hProtocol::UntrackDna(_msg) => {
                // FIXME
            }
            Lib3hProtocol::SendDirectMessage(_msg) => {
                // FIXME
            }
            Lib3hProtocol::HandleSendDirectMessageResult(_msg) => {
                // FIXME
            }
            Lib3hProtocol::FetchEntry(_msg) => {
                // FIXME
            }
            Lib3hProtocol::HandleFetchEntryResult(_msg) => {
                // FIXME
            }
            Lib3hProtocol::PublishEntry(_msg) => {
                // FIXME
            }
            Lib3hProtocol::FetchMeta(_msg) => {
                // FIXME
            }
            Lib3hProtocol::HandleFetchMetaResult(_msg) => {
                // FIXME
            }
            Lib3hProtocol::PublishMeta(_msg) => {
                // FIXME
            }
            // Our request for the publish_list has returned
            Lib3hProtocol::HandleGetPublishingEntryListResult(_msg) => {
                // FIXME
            }
            // Our request for the hold_list has returned
            Lib3hProtocol::HandleGetHoldingEntryListResult(_msg) => {
                // FIXME
            }
            // Our request for the publish_meta_list has returned
            Lib3hProtocol::HandleGetPublishingMetaListResult(_msg) => {
                // FIXME
            }
            // Our request for the hold_meta_list has returned
            Lib3hProtocol::HandleGetHoldingMetaListResult(_msg) => {
                // FIXME
            }
            _ => {
                panic!("unexpected {:?}", &local_msg);
            }
        }
        Ok((did_work, outbox))
    }

    /// Serve a P2pEvent sent to us
    /// Return a list of Lib3hProtocol to send to owner?
    // FIXME
    fn serve_P2pEvent(&mut self, evt: P2pEvent) -> Lib3hResult<Vec<Lib3hProtocol>> {
        let mut outbox = Vec::new();
        match evt {
            P2pEvent::HandleRequest(_data) => {
                // FIXME
            },
            P2pEvent::HandlePublish(_data) => {
                // FIXME
            },
            P2pEvent::PeerConnected(_peer_address) => {
                // FIXME
            },
            P2pEvent::PeerDisconnected(_peer_address) => {
                // FIXME
            },
        };
        Ok(outbox)
    }
}

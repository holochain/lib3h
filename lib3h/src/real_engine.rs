#![allow(non_snake_case)]

use std::collections::{HashMap, VecDeque};

use holochain_lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol, Address, DidWork, Lib3hResult,
};

use crate::{
    dht::{
        dht_event::{DhtEvent, PeerHoldRequestData},
        dht_trait::Dht,
    },
    p2p::{p2p_gateway::P2pGateway, p2p_protocol::P2pProtocol},
    transport::transport_trait::Transport,
};

/// Identifier of a source chain: DnaAddress+AgentId
pub type ChainId = (Address, Address);

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
    _config: RealEngineConfig,
    /// FIFO of Lib3hClientProtocol messages received from Core
    inbox: VecDeque<Lib3hClientProtocol>,
    /// Identifier
    name: String,
    /// P2p gateway for the transport layer,
    transport_gateway: P2pGateway,
    /// Map of P2p gateway per tracked DNA (per Agent?)
    dna_gateway_map: HashMap<ChainId, P2pGateway>,
}

impl RealEngine {
    /// Constructor
    pub fn new(config: RealEngineConfig, name: &str) -> Lib3hResult<Self> {
        Ok(RealEngine {
            _config: config,
            inbox: VecDeque::new(),
            name: name.to_string(),
            transport_gateway: P2pGateway::new(false),
            dna_gateway_map: HashMap::new(),
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

    /// Add incoming Lib3hClientProtocol message in FIFO
    fn post(&mut self, client_msg: Lib3hClientProtocol) -> Lib3hResult<()> {
        // println!("(log.t) RealEngine.post(): {:?}", client_msg);
        self.inbox.push_back(client_msg);
        Ok(())
    }

    /// Process Lib3hClientProtocol message inbox and
    /// output a list of Lib3hServerProtocol messages for Core to handle
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        // println!("(log.t) RealEngine.process()");
        // Process all received Lib3hClientProtocol messages from Core
        let (did_work, mut outbox) = self.process_inbox()?;
        // Process the transport layer
        let _ = self.process_transport_gateway()?;
        // Process all dna dhts
        let p2p_output = self.process_dna_gateways()?;
        // Process all generated P2pProtocol messages
        let mut output = self.process_p2p(&p2p_output)?;
        outbox.append(&mut output);
        // Done
        Ok((did_work, outbox))
    }
}

/// Private
impl RealEngine {
    /// Progressively serve every Lib3hClientProtocol received in inbox
    fn process_inbox(&mut self) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        let mut outbox = Vec::new();
        let mut did_work = false;
        loop {
            let client_msg = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            let (success, mut output) = self.serve_Lib3hProtocol(client_msg)?;
            if success {
                did_work = success;
            }
            outbox.append(&mut output);
        }
        Ok((did_work, outbox))
    }

    /// Progressively serve every P2pProtocol received in inbox
    fn process_transport_gateway(&mut self) -> Lib3hResult<DidWork> {
        let (did_work, p2p_list) = self.transport_gateway.do_process()?;
        if !did_work {
            return Ok(false);
        }
        for p2p_msg in p2p_list {
            self.serve_P2pProtocol(&p2p_msg)?;
        }
        Ok(true)
    }

    /// Process all dna gateways
    fn process_dna_gateways(&mut self) -> Lib3hResult<Vec<P2pProtocol>> {
        // Process all dna P2ps and store 'generated' P2pProtocol messages.
        let mut output = Vec::new();
        for (_dna_address, mut dna_p2p) in self.dna_gateway_map.iter_mut() {
            let (did_work, mut p2p_list) = dna_p2p.do_process()?;
            if did_work {
                output.append(&mut p2p_list);
            }
        }
        Ok(output)
    }
    /// Process all dna gateways
    fn process_p2p(&mut self, input: &Vec<P2pProtocol>) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        // Serve all new P2pProtocols
        let mut output = Vec::new();
        for p2p_msg in input {
            let mut evt_output = self.serve_P2pProtocol(p2p_msg)?;
            output.append(&mut evt_output);
        }
        Ok(output)
    }
    /// Serve a transportEvent sent to us.
    /// Return a list of TransportEvents for us to process.
    // FIXME
    fn serve_P2pProtocol(
        &mut self,
        p2p_msg: &P2pProtocol,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        let mut outbox = Vec::new();
        match p2p_msg {
            P2pProtocol::Gossip => {
                // FIXME
            }
            P2pProtocol::DirectMessage => {
                // FIXME
            }
            P2pProtocol::FetchData => {
                // FIXME
            }
            P2pProtocol::FetchDataResponse => {
                // FIXME
            }
        };
        Ok(outbox)
    }

    /// Process a Lib3hClientProtocol message sent to us (by Core)
    /// Return a list of Lib3hServerProtocol messages to send back to core or others?
    fn serve_Lib3hProtocol(
        &mut self,
        client_msg: Lib3hClientProtocol,
    ) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        println!(
            "(log.d) >>>> '{}' recv: {:?}",
            self.name.clone(),
            client_msg
        );
        let mut outbox = Vec::new();
        let mut did_work = true;
        // Note: use same order as the enum
        match client_msg {
            Lib3hClientProtocol::SuccessResult(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::FailureResult(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::Connect(msg) => {
                self.transport_gateway.connect(&msg.peer_transport)?;
            }
            Lib3hClientProtocol::TrackDna(msg) => {
                let output = self.serve_TrackDna(&msg)?;
                outbox.push(output);
            }
            Lib3hClientProtocol::UntrackDna(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::SendDirectMessage(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::HandleSendDirectMessageResult(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::FetchEntry(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::HandleFetchEntryResult(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::PublishEntry(_msg) => {
                // FIXME
            }
            // Our request for the publish_list has returned
            Lib3hClientProtocol::HandleGetPublishingEntryListResult(_msg) => {
                // FIXME
            }
            // Our request for the hold_list has returned
            Lib3hClientProtocol::HandleGetHoldingEntryListResult(_msg) => {
                // FIXME
            }
        }
        Ok((did_work, outbox))
    }

    /// Create DNA gateway for this agent if not already tracking
    fn serve_TrackDna(&mut self, track_msg: &TrackDnaData) -> Lib3hResult<Lib3hServerProtocol> {
        let chain_id = (track_msg.dna_address.clone(), track_msg.agent_id.clone());
        let mut res = ResultData {
            request_id: track_msg.request_id.clone(),
            dna_address: track_msg.dna_address.clone(),
            to_agent_id: track_msg.agent_id.clone(),
            result_info: vec![],
        };
        if self.dna_gateway_map.contains_key(&chain_id) {
            res.result_info = "Already tracked".to_string().into_bytes();
            return Ok(Lib3hServerProtocol::FailureResult(res));
        }
        self.dna_gateway_map
            .insert(chain_id.clone(), P2pGateway::new(true));
        let mut dna_p2p = self.dna_gateway_map.get_mut(&chain_id).unwrap();
        Dht::post(
            dna_p2p,
            DhtEvent::PeerHoldRequest(PeerHoldRequestData {
                peer_address: "FIXME".to_string(), // msg.agent_id,
                transport: self.transport_gateway.id(),
                timestamp: 42,
            }),
        )?;
        Ok(Lib3hServerProtocol::SuccessResult(res))
    }
}

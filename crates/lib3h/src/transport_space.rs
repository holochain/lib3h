use crate::transport::{
    error::TransportResult,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    TransportId, TransportIdRef,
};
use lib3h_protocol::DidWork;
use std::collections::VecDeque;

/// Transport used for the Space P2pGateway
pub struct TransportSpace<'a, 'b> {
    inbox: VecDeque<TransportCommand>,
    transport_gateway: &'a P2pGateway,
    dht: &'b impl Dht,
}

impl<'a, 'b>  TransportSpace<'a, 'b>  {
    pub fn new(transport_gateway: &'a P2pGateway, dht: &'b impl Dht) -> Self {
        TransportSpace {
            inbox: VecDeque::new(),
            transport_gateway,
            dht,
        }
    }
}

/// Compose Transport
impl Transport for TransportSpace {
    /// transport_id are agentId here
    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        let agent_list = self.dht.get_peer_list().map(|peer_info| peer_info.peer_address.clone());
        Ok(agent_list)
    }

    fn connect(&mut self, uri: &str) -> TransportResult<TransportId> {
        // uri is an AgentId
        Ok("FIXME".to_string())
    }
    fn close(&mut self, _id: &TransportIdRef) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    fn close_all(&mut self) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    /// Get MachineId out of agentId by asking the DHT's peer info
    /// If one agentId is unknown, will not send to any peer
    fn send(&mut self, id_list: &[&TransportIdRef], payload: &[u8]) -> TransportResult<()> {
        // Get all machineIds
        let mut machine_list = self.get_peer_info(id_list)?.map(|peer_info| peer_info.transport.clone());
        // Send payload to all peers via transport_gateway
        self.transport_gateway.post(TransportCommand::Send(&machine_list, payload))?;
        Ok(())
    }

    /// Get all known peers from DHT and send them the payload
    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        // Get all machineIds
        let mut machine_list = self.dht.get_peer_list().map(|peer_info| peer_info.transport.clone());
        // Send payload to all peers via transport_gateway
        self.transport_gateway.post(TransportCommand::Send(&machine_list, payload))?;
        Ok(())
    }

    ///
    fn bind(&mut self, _url: &str) -> TransportResult<String> {
        // FIXME
        Ok(String::new())
    }

    /// Add to inbox
    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.inbox.push_back(command);
        Ok(())
    }

    /// Process inbox
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        let outbox = Vec::new();
        let mut did_work = false;
        loop {
            let evt = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            println!("(log.t) TransportSpace.process(): {:?}", evt);
            let res = self.serve_TransportCommand(&cmd);
            if let Ok(mut output) = res {
                did_work = true;
                outbox.append(&mut output);
            }
        }
        Ok((did_work, outbox))
    }
}

impl TransportSpace {


    fn get_peer_info(&self, id_list: &[&TransportIdRef]) -> TransportResult<Vec<PeerHoldRequestData>> {
        // Get all machineIds
        let mut peer_info_list = Vec::new();
        for agent_id in id_list {
            let peer_info = self.dht.get_peer(agent_id).ok_or(
                Err(format_err!("AgentId is unknown")))?;
            machine_list.push(machine_list);
        }
        Ok(peer_info_list)
    }

    /// Process a TransportCommand: Call the corresponding method and possibily return some Events.
    /// Return a list of TransportEvents to owner.
    #[allow(non_snake_case)]
    fn serve_TransportCommand(
        &mut self,
        cmd: &TransportCommand,
    ) -> TransportResult<Vec<TransportEvent>> {
        println!("(log.d) >>> '(TransportSpace)' recv cmd: {:?}", cmd);
        // Note: use same order as the enum
        match cmd {
            TransportCommand::Connect(url) => {
                let id = self.connect(url)?;
                let evt = TransportEvent::ConnectResult(id);
                Ok(vec![evt])
            }
            TransportCommand::Send(id_list, payload) => {
                let mut id_ref_list = Vec::with_capacity(id_list.len());
                for id in id_list {
                    id_ref_list.push(id.as_str());
                }
                let _id = self.send(&id_ref_list, payload)?;
                Ok(vec![])
            }
            TransportCommand::SendAll(payload) => {
                let _id = self.send_all(payload)?;
                Ok(vec![])
            }
            TransportCommand::Close(id) => {
                self.close(id)?;
                let evt = TransportEvent::Closed(id.to_string());
                Ok(vec![evt])
            }
            TransportCommand::CloseAll => {
                self.close_all()?;
                let mut outbox = Vec::new();
                for (id, _url) in &self.connections {
                    let evt = TransportEvent::Closed(id.to_string());
                    outbox.push(evt);
                }
                Ok(outbox)
            }
            TransportCommand::Bind(url) => {
                self.bind(url)?;
                Ok(vec![])
            }
        }
    }
}
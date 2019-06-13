use crate::{
    dht::dht_protocol::PeerData,
    transport::{
        error::TransportResult,
        protocol::{TransportCommand, TransportEvent},
        transport_trait::Transport,
        TransportId, TransportIdRef,
    },
};
use lib3h_protocol::DidWork;
use std::collections::VecDeque;

/// Transport used for the Space P2pGateway
pub struct TransportSpace {
    inbox: VecDeque<TransportCommand>,
}

impl TransportSpace {
    pub fn new() -> Self {
        TransportSpace {
            inbox: VecDeque::new(),
        }
    }
}

/// Compose Transport
impl Transport for TransportSpace {
    /// transport_id are agentId here
    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        // FIXME
        Ok(vec![])
    }

    fn connect(&mut self, _uri: &str) -> TransportResult<TransportId> {
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
    fn send(&mut self, _id_list: &[&TransportIdRef], _payload: &[u8]) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    /// Get all known peers from DHT and send them the payload
    fn send_all(&mut self, _payload: &[u8]) -> TransportResult<()> {
        // FIXME
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
        let mut outbox = Vec::new();
        let mut did_work = false;
        loop {
            let cmd = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            println!("(log.t) TransportSpace.process(): {:?}", cmd);
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
    fn _get_peer_info(&self, _id_list: &[&TransportIdRef]) -> TransportResult<Vec<PeerData>> {
        // Get all machineIds
        let peer_info_list = Vec::new();
        // FIXME
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
                let outbox = Vec::new();
                // FIXME
                Ok(outbox)
            }
            TransportCommand::Bind(url) => {
                self.bind(url)?;
                Ok(vec![])
            }
        }
    }
}

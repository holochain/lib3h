use crate::transport::{
    error::TransportResult,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    TransportId, TransportIdRef,
};
use holochain_lib3h_protocol::DidWork;
use std::collections::VecDeque;

/// Transport used for the DNA P2pGateway
pub struct TransportDna {
    inbox: VecDeque<TransportCommand>,
}

impl TransportDna {
    pub fn new() -> Self {
        TransportDna {
            inbox: VecDeque::new(),
        }
    }
}

/// Compose Transport
impl Transport for TransportDna {
    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        // FIXME
        Ok(vec![])
    }

    fn connect(&mut self, _uri: &str) -> TransportResult<TransportId> {
        // FIXME
        Ok("FIXME".to_string())
    }
    fn close(&mut self, _id: TransportId) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    fn close_all(&mut self) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    fn send(&mut self, _id_list: &[&TransportIdRef], _payload: &[u8]) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    fn send_all(&mut self, _payload: &[u8]) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.inbox.push_back(command);
        Ok(())
    }

    // FIXME
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        let outbox = Vec::new();
        let mut did_work = false;
        loop {
            let evt = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            did_work = true;
            println!("(log.t) TransportDna.process(): {:?}", evt)
        }
        Ok((did_work, outbox))
    }
}

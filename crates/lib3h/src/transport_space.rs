use crate::transport::{
    error::TransportResult,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    TransportId, TransportIdRef,
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
    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        // FIXME
        Ok(vec![])
    }

    fn connect(&mut self, _uri: &str) -> TransportResult<TransportId> {
        // FIXME
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

    fn bind(&mut self, _url: &str) -> TransportResult<String> {
        // FIXME
        Ok(String::new())
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
            println!("(log.t) TransportSpace.process(): {:?}", evt)
        }
        Ok((did_work, outbox))
    }
}

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

    fn post(&mut self, _command: TransportCommand) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        // FIXME
        Ok((false, vec![]))
    }
}

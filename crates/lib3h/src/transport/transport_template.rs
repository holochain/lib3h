use crate::transport::{
    error::TransportResult, protocol::*, transport_trait::Transport,
};
use lib3h_protocol::DidWork;
use std::collections::VecDeque;
use url::Url;

pub struct TransportTemplate {
    cmd_inbox: VecDeque<TransportCommand>,
    evt_outbox: Vec<TransportEvent>,
}

impl TransportTemplate {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            cmd_inbox: VecDeque::new(),
            evt_outbox: Vec::new(),
        }
    }

    fn priv_bind(&mut self, _request_id: RequestId, _spec: Url) -> TransportResult<()> {
        unimplemented!()
    }

    fn priv_connect(&mut self, _request_id: RequestId, _address: Url) -> TransportResult<()> {
        unimplemented!()
    }

    fn priv_send(
        &mut self,
        _origin_stack: String,
        _request_id: RequestId,
        _address: Url,
        _payload: Vec<u8>,
    ) -> TransportResult<()> {
        unimplemented!()
    }

    fn priv_handle_command(&mut self, cmd: TransportCommand) -> TransportResult<()> {
        match cmd {
            TransportCommand::Bind { request_id, spec } => self.priv_bind(request_id, spec),
            TransportCommand::Connect {
                request_id,
                address,
            } => self.priv_connect(request_id, address),
            TransportCommand::SendMessage {
                origin_stack,
                request_id,
                address,
                payload,
            } => self.priv_send(origin_stack, request_id, address, payload),
        }
    }
}

impl Transport for TransportTemplate {
    fn connection_list(&self) -> TransportResult<Vec<Url>> {
        unimplemented!()
    }

    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.cmd_inbox.push_back(command);
        Ok(())
    }

    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        let mut did_work = false;

        loop {
            let cmd = match self.cmd_inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            did_work = true;
            self.priv_handle_command(cmd)?;
        }

        Ok((did_work, self.evt_outbox.drain(..).collect()))
    }

    fn bind_sync(&mut self, _spec: Url) -> TransportResult<Url> {
        unimplemented!()
    }
}

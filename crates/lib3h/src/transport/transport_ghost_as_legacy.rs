use crate::transport::{error::*, protocol::*, ConnectionId, ConnectionIdRef, Transport};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::DidWork;
use url::Url;

#[derive(Debug)]
enum ToInnerContext {
    None,
}

pub struct TransportGhostAsLegacy<'lt> {
    // ref to our inner transport
    inner_transport:
        Detach<TransportActorParentWrapperDyn<'lt, TransportGhostAsLegacy<'lt>, ToInnerContext>>,
    outbox: Vec<TransportEvent>,
    sync_ready: bool,
    bound_url: Url,
}

impl<'lt> TransportGhostAsLegacy<'lt> {
    /// create a new TransportGhostAsLegacy Instance
    pub fn new(inner_transport: DynTransportActor<'lt>) -> Self {
        let inner_transport = Detach::new(GhostParentWrapperDyn::new(
            inner_transport,
            "mplex_to_inner_",
        ));
        Self {
            inner_transport,
            outbox: Vec::new(),
            sync_ready: false,
            bound_url: Url::parse("none:").expect("can parse url"),
        }
    }

    /// private dispatcher for messages from our inner transport
    fn handle_msg_from_inner(
        &mut self,
        mut msg: GhostMessage<
            RequestToParent,
            RequestToChild,
            RequestToParentResponse,
            TransportError,
        >,
    ) -> TransportResult<()> {
        match msg.take_message().expect("exists") {
            RequestToParent::IncomingConnection { address } => {
                self.handle_incoming_connection(address)
            }
            RequestToParent::ReceivedData { address, payload } => {
                self.handle_received_data(address, payload)
            }
            RequestToParent::TransportError { error } => self.handle_transport_error(error),
        }
    }

    /// private handler for inner transport IncomingConnection events
    fn handle_incoming_connection(&mut self, address: Url) -> TransportResult<()> {
        // forward
        self.outbox
            .push(TransportEvent::IncomingConnectionEstablished(
                address.to_string(),
            ));
        Ok(())
    }

    /// private handler for inner transport ReceivedData events
    fn handle_received_data(&mut self, address: Url, payload: Vec<u8>) -> TransportResult<()> {
        // forward
        self.outbox
            .push(TransportEvent::ReceivedData(address.to_string(), payload));
        Ok(())
    }

    /// private handler for inner transport TransportError events
    fn handle_transport_error(&mut self, error: TransportError) -> TransportResult<()> {
        error!("cannot handle transport errors here: {:?}", error);
        Ok(())
    }

    fn priv_process(&mut self) -> TransportResult<()> {
        detach_run!(&mut self.inner_transport, |it| it.process(self))?;
        for msg in self.inner_transport.as_mut().drain_messages() {
            self.handle_msg_from_inner(msg)?;
        }
        Ok(())
    }

    /// private handler for Bind requests from our parent
    fn priv_bind(&mut self, spec: Url) -> TransportResult<Url> {
        self.sync_ready = false;

        // forward the bind to our inner_transport
        self.inner_transport.as_mut().request(
            ToInnerContext::None,
            RequestToChild::Bind { spec },
            Box::new(|me, _, response| {
                me.sync_ready = true;
                let response = {
                    match response {
                        GhostCallbackData::Timeout => {
                            return Err("timeout".into());
                        }
                        GhostCallbackData::Response(response) => response,
                    }
                };
                match response {
                    Ok(RequestToChildResponse::Bind(BindResultData { bound_url })) => {
                        me.bound_url = bound_url;
                    }
                    _ => return Err("invalid".into()),
                }
                Ok(())
            }),
        )?;

        loop {
            self.priv_process()?;
            if self.sync_ready {
                break;
            }
        }

        Ok(self.bound_url.clone())
    }

    /// private handler for SendMessage requests from our parent
    fn priv_send_message(&mut self, address: Url, payload: Vec<u8>) -> TransportResult<()> {
        self.sync_ready = false;

        // forward the request to our inner_transport
        self.inner_transport.as_mut().request(
            ToInnerContext::None,
            RequestToChild::SendMessage { address, payload },
            Box::new(|me, _, _| {
                me.sync_ready = true;
                Ok(())
            }),
        )?;

        loop {
            self.priv_process()?;
            if self.sync_ready {
                break;
            }
        }

        Ok(())
    }
}

impl<'lt> Transport for TransportGhostAsLegacy<'lt> {
    fn connect(&mut self, uri: &Url) -> TransportResult<ConnectionId> {
        warn!("@@@ - GAL:connect({})", &uri);
        // psyche!
        Ok(uri.to_string())
    }

    fn close(&mut self, _id: &ConnectionIdRef) -> TransportResult<()> {
        // no-op
        Ok(())
    }

    fn close_all(&mut self) -> TransportResult<()> {
        // no-op
        Ok(())
    }

    fn send(&mut self, id_list: &[&ConnectionIdRef], payload: &[u8]) -> TransportResult<()> {
        for id in id_list {
            warn!("@@@ - GAL:send({})", &id);
            let fake_uri = Url::parse(&format!("fake:{}", id)).expect("can parse url");
            self.priv_send_message(fake_uri, payload.to_vec())?;
        }
        Ok(())
    }

    fn send_all(&mut self, _payload: &[u8]) -> TransportResult<()> {
        error!("cannot send_all");
        Ok(())
    }

    fn bind(&mut self, url: &url::Url) -> TransportResult<Url> {
        self.priv_bind(url.clone())
    }

    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        match command {
            TransportCommand::Connect(url, _rid) => {
                self.connect(&url)?;
            }
            TransportCommand::Send(id_list, payload) => {
                self.send(
                    &id_list.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
                    &payload,
                )?;
            }
            TransportCommand::SendAll(payload) => {
                self.send_all(&payload)?;
            }
            TransportCommand::Close(id) => {
                self.close(&id)?;
            }
            TransportCommand::CloseAll => {
                self.close_all()?;
            }
            TransportCommand::Bind(url) => {
                self.bind(&url)?;
            }
        };
        Ok(())
    }

    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        self.priv_process()?;
        Ok((true, self.outbox.drain(..).collect()))
    }

    fn connection_id_list(&mut self) -> TransportResult<Vec<ConnectionId>> {
        Ok(Vec::new())
    }

    fn get_uri(&self, id: &ConnectionIdRef) -> Option<Url> {
        // psyche!
        match Url::parse(id) {
            Err(_) => None,
            Ok(address) => Some(address),
        }
    }
}

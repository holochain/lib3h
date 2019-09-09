use crate::transport::{error::*, protocol::*, ConnectionId};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use std::collections::HashMap;
use url::Url;

use super::TransportWrapper;

pub struct TransportLegacyAsGhost<'gt> {
    // our parent channel endpoint
    endpoint_parent: Option<TransportActorParentEndpoint>,
    // our self channel endpoint
    endpoint_self: Detach<TransportActorSelfEndpoint<TransportLegacyAsGhost<'gt>, ()>>,
    // ref to our inner legacy transport
    inner_transport: TransportWrapper<'gt>,
    url_to_con_id: HashMap<Url, ConnectionId>,
    con_id_to_url: HashMap<ConnectionId, Url>,
}

impl<'gt> TransportLegacyAsGhost<'gt> {
    /// create a new TransportLegacyAsGhost Instance
    pub fn new(inner_transport: TransportWrapper<'gt>) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(
            endpoint_self
                .as_context_endpoint_builder()
                .request_id_prefix("mplex_to_parent_")
                .build(),
        );
        Self {
            endpoint_parent,
            endpoint_self,
            inner_transport,
            url_to_con_id: HashMap::new(),
            con_id_to_url: HashMap::new(),
        }
    }

    /// private dispatcher for messages coming from our parent
    fn handle_msg_from_parent(
        &mut self,
        mut msg: GhostMessage<
            RequestToChild,
            RequestToParent,
            RequestToChildResponse,
            TransportError,
        >,
    ) -> TransportResult<()> {
        match msg.take_message().expect("exists") {
            RequestToChild::Bind { spec } => self.handle_bind(msg, spec),
            RequestToChild::SendMessage { address, payload } => {
                self.handle_send_message(msg, address, payload)
            }
        }
    }

    /// private handler for Bind requests from our parent
    fn handle_bind(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        spec: Url,
    ) -> TransportResult<()> {
        // forward the bind to our inner_transport
        let bound_url = self.inner_transport.as_mut().bind(&spec)?;
        warn!("@@@ - LAG:BIND({})->{}", &spec, &bound_url);
        msg.respond(Ok(RequestToChildResponse::Bind(BindResultData {
            bound_url,
        })))?;
        Ok(())
    }

    /// private handler for SendMessage requests from our parent
    fn handle_send_message(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        address: Url,
        payload: Vec<u8>,
    ) -> TransportResult<()> {
        if !self.url_to_con_id.contains_key(&address) {
            // connect if needed
            let con_id = self.inner_transport.as_mut().connect(&address)?;
            self.url_to_con_id.insert(address.clone(), con_id.clone());
            self.con_id_to_url.insert(con_id.clone(), address.clone());
        }
        let con_id = self.url_to_con_id.get(&address).ok_or(TransportError::from(format!("error connecting to {:?}", address)))?.clone();
        warn!("@@@ - LAG:SEND({}->{})", &address, &con_id);
        // forward the request to our inner_transport
        self.inner_transport.as_mut().send(&[&con_id], &payload)?;
        msg.respond(Ok(RequestToChildResponse::SendMessage))?;
        Ok(())
    }
}

impl<'gt>
    GhostActor<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        TransportError,
    > for TransportLegacyAsGhost<'gt>
{
    fn take_parent_endpoint(&mut self) -> Option<TransportActorParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        warn!("@@@ - LAG:process concrete");
        detach_run!(&mut self.endpoint_self, |es| es.process(self))?;
        for msg in self.endpoint_self.as_mut().drain_messages() {
            self.handle_msg_from_parent(msg)?;
        }
        let (_did_work, events) = self.inner_transport.as_mut().process()?;
        for ev in events {
            match ev {
                TransportEvent::ErrorOccured(id, _e) => {
                    // just delete our interal tracking things
                    match self.con_id_to_url.remove(&id) {
                        None => (),
                        Some(address) => {
                            self.url_to_con_id.remove(&address);
                        }
                    }
                }
                TransportEvent::ConnectResult(_id, _r) => {
                    // ??
                }
                TransportEvent::IncomingConnectionEstablished(id) => {
                    let address = self.inner_transport.as_ref().get_uri(&id).ok_or(TransportError::from(format!("no address for con id {:?}", &id)))?;
                    warn!("@@@ - LAG:IN({}->{})", &id, &address);
                    self.con_id_to_url.insert(id.clone(), address.clone());
                    self.url_to_con_id.insert(address.clone(), id.clone());
                    self.endpoint_self
                        .publish(RequestToParent::IncomingConnection { address })?;
                }
                TransportEvent::ConnectionClosed(_id) => {}
                TransportEvent::ReceivedData(id, payload) => {
                    let address = self.inner_transport.as_ref().get_uri(&id).ok_or(TransportError::from(format!("no address for con id {:?}", &id)))?;
                    warn!("@@@ - LAG:RECV({}->{})", &id, &address);
                    self.con_id_to_url.insert(id.clone(), address.clone());
                    self.url_to_con_id.insert(address.clone(), id.clone());
                    self.endpoint_self
                        .publish(RequestToParent::ReceivedData { address, payload })?;
                }
            }
        }
        Ok(false.into())
    }
}

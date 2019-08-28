use crate::{
    keystore::*,
    transport::{error::*, protocol::*},
};
use detach::prelude::*;
use lib3h_crypto_api::CryptoSystem;
use lib3h_ghost_actor::prelude::*;
use std::{any::Any, collections::HashMap};
use url::Url;

enum ToParentContext {}

enum ToInnerContext {
    AwaitBind(
        GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
    ),
    AwaitSend(
        GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
    ),
}

enum ToKeystoreContext {}

pub struct TransportEncoding {
    #[allow(dead_code)]
    crypto: Box<dyn CryptoSystem>,
    this_id: String,
    keystore: Detach<KeystoreActorParentWrapper<ToKeystoreContext>>,
    endpoint_parent: Option<TransportActorParentEndpoint>,
    endpoint_self: Detach<
        GhostContextEndpoint<
            ToParentContext,
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            TransportError,
        >,
    >,
    inner_transport: Detach<TransportActorParentWrapper<ToInnerContext>>,
    #[allow(clippy::complexity)]
    pending_send_data: HashMap<
        Url,
        Vec<(
            Vec<u8>,
            GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        )>,
    >,
    pending_received_data: HashMap<Url, Vec<Vec<u8>>>,
    connections_no_id_to_id: HashMap<Url, Url>,
    connections_id_to_no_id: HashMap<Url, Url>,
}

impl TransportEncoding {
    pub fn new(
        crypto: Box<dyn CryptoSystem>,
        this_id: String,
        keystore: KeystoreActor,
        inner_transport: TransportActor,
    ) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(endpoint_self.as_context_endpoint("enc_to_parent_"));
        let keystore = Detach::new(GhostParentWrapper::new(keystore, "enc_to_keystore"));
        let inner_transport =
            Detach::new(GhostParentWrapper::new(inner_transport, "enc_to_inner_"));
        Self {
            crypto,
            this_id,
            keystore,
            endpoint_parent,
            endpoint_self,
            inner_transport,
            pending_send_data: HashMap::new(),
            pending_received_data: HashMap::new(),
            connections_no_id_to_id: HashMap::new(),
            connections_id_to_no_id: HashMap::new(),
        }
    }

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

    fn send_handshake(&mut self, address: &Url) {
        self.inner_transport.publish(RequestToChild::SendMessage {
            address: address.clone(),
            payload: self.this_id.as_bytes().to_vec(),
        });
    }

    fn handle_incoming_connection(&mut self, address: Url) -> TransportResult<()> {
        match self.connections_no_id_to_id.get(&address) {
            Some(remote_addr) => {
                self.endpoint_self
                    .publish(RequestToParent::IncomingConnection {
                        address: remote_addr.clone(),
                    });
            }
            None => {
                self.send_handshake(&address);
            }
        }
        Ok(())
    }

    fn handle_received_data(&mut self, address: Url, payload: Vec<u8>) -> TransportResult<()> {
        match self.connections_no_id_to_id.get(&address) {
            Some(remote_addr) => {
                self.endpoint_self.publish(RequestToParent::ReceivedData {
                    address: remote_addr.clone(),
                    payload,
                });
            }
            None => {
                if payload.len() == 63 && payload[0] == b'H' && payload[1] == b'c' {
                    let remote_id = String::from_utf8_lossy(&payload);
                    let mut remote_url = address.clone();
                    remote_url.query_pairs_mut().append_pair("a", &remote_id);
                    self.connections_no_id_to_id
                        .insert(address.clone(), remote_url.clone());
                    self.connections_id_to_no_id
                        .insert(remote_url.clone(), address.clone());
                    self.endpoint_self
                        .publish(RequestToParent::IncomingConnection {
                            address: remote_url.clone(),
                        });
                    if let Some(items) = self.pending_received_data.remove(&address) {
                        for payload in items {
                            self.endpoint_self.publish(RequestToParent::ReceivedData {
                                address: remote_url.clone(),
                                payload,
                            });
                        }
                    }
                    if let Some(items) = self.pending_send_data.remove(&address) {
                        for (payload, msg) in items {
                            self.fwd_send_message_result(msg, address.clone(), payload)?;
                        }
                    }
                } else {
                    self.send_handshake(&address);
                    let e = self
                        .pending_received_data
                        .entry(address)
                        .or_insert_with(|| vec![]);
                    e.push(payload);
                }
            }
        }
        Ok(())
    }

    fn handle_transport_error(&mut self, error: TransportError) -> TransportResult<()> {
        self.endpoint_self
            .publish(RequestToParent::TransportError { error });
        Ok(())
    }

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

    fn handle_bind(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        spec: Url,
    ) -> TransportResult<()> {
        self.inner_transport.as_mut().request(
            std::time::Duration::from_millis(2000),
            ToInnerContext::AwaitBind(msg),
            RequestToChild::Bind { spec },
            Box::new(|m, context, response| {
                let m = match m.downcast_mut::<TransportEncoding>() {
                    None => panic!("wrong type"),
                    Some(m) => m,
                };
                let msg = {
                    match context {
                        ToInnerContext::AwaitBind(msg) => msg,
                        _ => panic!("bad context"),
                    }
                };
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => match response {
                            Err(e) => panic!("{:?}", e),
                            Ok(response) => response,
                        },
                    }
                };
                if let RequestToChildResponse::Bind(mut data) = response {
                    data.bound_url
                        .query_pairs_mut()
                        .append_pair("a", &m.this_id);
                    info!("got bind response: {:?}", data.bound_url);
                    msg.respond(Ok(RequestToChildResponse::Bind(data)));
                } else {
                    panic!("bad response to bind: {:?}", response);
                }
                Ok(())
            }),
        );
        Ok(())
    }

    fn fwd_send_message_result(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        address: Url,
        payload: Vec<u8>,
    ) -> TransportResult<()> {
        self.inner_transport.as_mut().request(
            std::time::Duration::from_millis(2000),
            ToInnerContext::AwaitSend(msg),
            RequestToChild::SendMessage { address, payload },
            Box::new(|_, context, response| {
                let msg = {
                    match context {
                        ToInnerContext::AwaitSend(msg) => msg,
                        _ => panic!("bad context"),
                    }
                };
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => response,
                    }
                };
                msg.respond(response);
                Ok(())
            }),
        );
        Ok(())
    }

    fn handle_send_message(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        address: Url,
        payload: Vec<u8>,
    ) -> TransportResult<()> {
        match self.connections_id_to_no_id.get(&address) {
            Some(sub_address) => {
                let sub_address = sub_address.clone();
                self.fwd_send_message_result(msg, sub_address, payload)?;
            }
            None => {
                let mut sub_address = address.clone();
                sub_address.query_pairs_mut().clear();
                self.send_handshake(&sub_address);
                let e = self
                    .pending_send_data
                    .entry(sub_address)
                    .or_insert_with(|| vec![]);
                e.push((payload, msg));
            }
        }
        Ok(())
    }
}

impl
    GhostActor<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        TransportError,
    > for TransportEncoding
{
    fn as_any(&mut self) -> &mut dyn Any {
        &mut *self
    }

    fn take_parent_endpoint(&mut self) -> Option<TransportActorParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        detach_run!(&mut self.endpoint_self, |es| es.process(self.as_any()))?;
        for msg in self.endpoint_self.as_mut().drain_messages() {
            self.handle_msg_from_parent(msg)?;
        }
        detach_run!(&mut self.inner_transport, |it| it.process(self.as_any()))?;
        for msg in self.inner_transport.as_mut().drain_messages() {
            self.handle_msg_from_inner(msg)?;
        }
        detach_run!(&mut self.keystore, |ks| ks.process(self.as_any()))?;
        Ok(false.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib3h_sodium::SodiumCryptoSystem;

    const ID_1: &'static str = "HcSCJ9G64XDKYo433rIMm57wfI8Y59Udeb4hkVvQBZdm6bgbJ5Wgs79pBGBcuzz";
    const ID_2: &'static str = "HcMCJ8HpYvB4zqic93d3R4DjkVQ4hhbbv9UrZmWXOcn3m7w4O3AIr56JRfrt96r";

    pub struct TransportMock {
        endpoint_parent: Option<TransportActorParentEndpoint>,
        endpoint_self: Detach<
            GhostContextEndpoint<
                ToParentContext,
                RequestToParent,
                RequestToParentResponse,
                RequestToChild,
                RequestToChildResponse,
                TransportError,
            >,
        >,
    }

    impl TransportMock {
        pub fn new() -> Self {
            let (endpoint_parent, endpoint_self) = create_ghost_channel();
            let endpoint_parent = Some(endpoint_parent);
            let endpoint_self = Detach::new(endpoint_self.as_context_endpoint("mock_to_parent_"));
            Self {
                endpoint_parent,
                endpoint_self,
            }
        }
    }

    impl
        GhostActor<
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            TransportError,
        > for TransportMock
    {
        fn as_any(&mut self) -> &mut dyn Any {
            &mut *self
        }

        fn take_parent_endpoint(&mut self) -> Option<TransportActorParentEndpoint> {
            std::mem::replace(&mut self.endpoint_parent, None)
        }

        fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
            detach_run!(&mut self.endpoint_self, |es| es.process(self.as_any()))?;
            for mut msg in self.endpoint_self.as_mut().drain_messages() {
                match msg.take_message().expect("exists") {
                    RequestToChild::Bind { mut spec } => {
                        spec.set_path("bound");
                        msg.respond(Ok(RequestToChildResponse::Bind(BindResultData {
                            bound_url: spec,
                        })));
                    }
                    RequestToChild::SendMessage {
                        address: _,
                        payload: _,
                    } => {
                        msg.respond(Ok(RequestToChildResponse::SendMessage));
                    }
                }
            }
            Ok(false.into())
        }
    }

    #[test]
    fn it_should_exchange_messages() {
        let crypto: Box<dyn CryptoSystem> =
            Box::new(SodiumCryptoSystem::new().set_pwhash_interactive());

        let mut t1: TransportActorParentWrapper<()> = GhostParentWrapper::new(
            Box::new(TransportEncoding::new(
                crypto.box_clone(),
                ID_1.to_string(),
                Box::new(KeystoreStub::new()),
                Box::new(TransportMock::new()),
            )),
            "test1",
        );

        t1.request(
            std::time::Duration::from_millis(2000),
            (),
            RequestToChild::Bind {
                spec: Url::parse("test://1").expect("can parse url"),
            },
            Box::new(|_, _, response| {
                assert_eq!(
                    &format!("{:?}", response),
                    "Response(Ok(Bind(BindResultData { bound_url: \"test://1/bound?a=HcSCJ9G64XDKYo433rIMm57wfI8Y59Udeb4hkVvQBZdm6bgbJ5Wgs79pBGBcuzz\" })))"
                );
                Ok(())
            })
        );

        t1.process(&mut ()).unwrap();

        let mut t2: TransportActorParentWrapper<()> = GhostParentWrapper::new(
            Box::new(TransportEncoding::new(
                crypto.box_clone(),
                ID_2.to_string(),
                Box::new(KeystoreStub::new()),
                Box::new(TransportMock::new()),
            )),
            "test2",
        );

        t2.request(
            std::time::Duration::from_millis(2000),
            (),
            RequestToChild::Bind {
                spec: Url::parse("test://2").expect("can parse url"),
            },
            Box::new(|_, _, response| {
                assert_eq!(
                    &format!("{:?}", response),
                    "Response(Ok(Bind(BindResultData { bound_url: \"test://2/bound?a=HcMCJ8HpYvB4zqic93d3R4DjkVQ4hhbbv9UrZmWXOcn3m7w4O3AIr56JRfrt96r\" })))"
                );
                Ok(())
            })
        );

        t2.process(&mut ()).unwrap();
    }
}

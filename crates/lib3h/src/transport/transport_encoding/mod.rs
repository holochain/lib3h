use crate::{
    keystore::*,
    transport::{error::*, protocol::*},
};
use detach::prelude::*;
use holochain_tracing::Span;
use lib3h_crypto_api::CryptoSystem;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::Opaque, uri::Lib3hUri};
use std::collections::HashMap;

/// temporary protocol enum for wire encoding
#[derive(Debug, Clone, Serialize, Deserialize)]
enum InterimEncodingProtocol {
    Error {
        message: String,
    },
    Handshake {
        magic: u16,
        network_id: String,
        id: String,
    },
    Payload {
        payload: Opaque,
    },
}

impl InterimEncodingProtocol {
    fn to_vec(&self) -> Opaque {
        serde_json::to_vec_pretty(self).unwrap().into()
    }

    fn from_slice(v: &[u8]) -> Self {
        serde_json::from_slice(v).unwrap()
    }
}

/// Wraps a lower-level transport in either Open or Encrypted communication
/// Also adds a concept of TransportId and AgentId
/// This is currently a stub, only the Id concept is in place.
pub struct TransportEncoding {
    #[allow(dead_code)]
    crypto: Box<dyn CryptoSystem>,
    // the transport_id or agent_id of this encoding instance
    this_id: String,
    // the current network_id
    this_network_id: String,
    // the keystore to use for getting signatures for `this_id`
    keystore: Detach<KeystoreActorParentWrapperDyn>,
    // our parent channel endpoint
    endpoint_parent: Option<TransportActorParentEndpoint>,
    // our self channel endpoint
    endpoint_self: Detach<
        GhostContextEndpoint<
            Self,
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            TransportError,
        >,
    >,
    // ref to our inner transport
    inner_transport: Detach<TransportActorParentWrapperDyn<TransportEncoding>>,
    // if we have never sent a message to this node before,
    // we need to first handshake. Store the send payload && msg object
    // we will continue the transaction once the handshake completes
    #[allow(clippy::complexity)]
    pending_send_data: HashMap<
        Lib3hUri,
        Vec<(
            Opaque,
            GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        )>,
    >,
    // if we have never received data from this remote before, we need to
    // handshake... this should never exactly happen... we should get a
    // handshake as the first request they send
    pending_received_data: HashMap<Lib3hUri, Vec<Opaque>>,
    // map low-level connection addresses to id connection addresses
    // i.e. wss://1.1.1.1:55888 -> wss://1.1.1.1:55888?a=HcMyadayada
    connections_no_id_to_id: HashMap<Lib3hUri, Lib3hUri>,
    // map id connection addresses to low-level connection addresses
    // i.e. wss://1.1.1.1:55888?a=HcMyadayada -> wss://1.1.1.1:55888
    connections_id_to_no_id: HashMap<Lib3hUri, Lib3hUri>,
}

impl TransportEncoding {
    /// create a new TransportEncoding Instance
    pub fn new(
        crypto: Box<dyn CryptoSystem>,
        this_id: String,
        this_network_id: String,
        keystore: DynKeystoreActor,
        inner_transport: DynTransportActor,
    ) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(
            endpoint_self
                .as_context_endpoint_builder()
                .request_id_prefix("enc_to_parent_")
                .build(),
        );
        let keystore = Detach::new(GhostParentWrapperDyn::new(keystore, "enc_to_keystore"));
        let inner_transport =
            Detach::new(GhostParentWrapperDyn::new(inner_transport, "enc_to_inner_"));
        Self {
            crypto,
            this_id,
            this_network_id,
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
        // different name span for each arm?
        let span = msg.span().child("handle_msg_from_inner");
        match msg.take_message().expect("exists") {
            RequestToParent::IncomingConnection { uri } => {
                self.handle_incoming_connection(span, uri)
            }
            RequestToParent::ReceivedData { uri, payload } => {
                match self.handle_received_data(span, uri.clone(), payload) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        match self.connections_no_id_to_id.remove(&uri) {
                            Some(r) => {
                                self.connections_id_to_no_id.remove(&r);
                            }
                            _ => (),
                        };
                        Err(e)
                    }
                }
            }
            RequestToParent::ErrorOccured { uri, error } => {
                self.handle_transport_error(span, uri, error)
            }
        }
    }

    /// private send a handshake to a remote address
    fn send_handshake(&mut self, span: Span, uri: &Lib3hUri) -> GhostResult<()> {
        let payload = InterimEncodingProtocol::Handshake {
            magic: 0x1f6c,
            network_id: self.this_network_id.clone(),
            id: self.this_id.clone(),
        }
        .to_vec();
        self.inner_transport.publish(
            span,
            RequestToChild::SendMessage {
                uri: uri.clone(),
                payload,
            },
        )
    }

    /// send an error
    fn send_error(&mut self, span: Span, uri: &Lib3hUri, message: String) -> TransportResult<()> {
        let payload = InterimEncodingProtocol::Error {
            message: message.clone(),
        }
        .to_vec();
        self.inner_transport.publish(
            span,
            RequestToChild::SendMessage {
                uri: uri.clone(),
                payload,
            },
        )?;
        Err(message.into())
    }

    /// private handler for inner transport IncomingConnection events
    fn handle_incoming_connection(&mut self, span: Span, uri: Lib3hUri) -> TransportResult<()> {
        match self.connections_no_id_to_id.get(&uri) {
            Some(remote_addr) => {
                // if we've already seen this connection, just forward it?
                self.endpoint_self.publish(
                    span,
                    RequestToParent::IncomingConnection {
                        uri: remote_addr.clone(),
                    },
                )?;
            }
            None => {
                // we've never seen this connection, handshake before
                // forwarding the IncomingConnection msg
                // (see handle_recveived_data for where it's actually sent)
                self.send_handshake(span, &uri)?;
            }
        }
        Ok(())
    }

    /// private handler received handshake from remote
    fn handle_received_remote_handshake(
        &mut self,
        span: Span,
        uri: &Lib3hUri,
        remote_id: String,
    ) -> TransportResult<()> {
        // build a higher-level id address
        let mut remote_url = uri.clone();
        remote_url.query_pairs_mut().append_pair("a", &remote_id);

        // set up low->high and high->low mappings
        self.connections_no_id_to_id
            .insert(uri.clone(), remote_url.clone());
        self.connections_id_to_no_id
            .insert(remote_url.clone(), uri.clone());

        // forward an IncomingConnection event to our parent
        self.endpoint_self.publish(
            span.follower("TODO name"),
            RequestToParent::IncomingConnection {
                uri: remote_url.clone(),
            },
        )?;

        // if we have any pending received data, send it up
        if let Some(items) = self.pending_received_data.remove(uri) {
            for payload in items {
                self.endpoint_self.publish(
                    span.follower("TODO name"),
                    RequestToParent::ReceivedData {
                        uri: remote_url.clone(),
                        payload,
                    },
                )?;
            }
        }

        // if we have any pending send data, send it down
        if let Some(items) = self.pending_send_data.remove(uri) {
            for (payload, msg) in items {
                self.fwd_send_message_result(msg, uri.clone(), payload)?;
            }
        }

        Ok(())
    }

    /// private handler for inner transport ReceivedData events
    fn handle_received_data(
        &mut self,
        span: Span,
        uri: Lib3hUri,
        payload: Opaque,
    ) -> TransportResult<()> {
        match InterimEncodingProtocol::from_slice(&payload) {
            InterimEncodingProtocol::Error { message } => {
                return Err(message.into());
            }
            InterimEncodingProtocol::Handshake {
                magic,
                network_id,
                id,
            } => {
                if magic != 0x1f6c {
                    return self.send_error(span, &uri, format!("bad magic: {:?}", magic));
                }
                if &network_id != &self.this_network_id {
                    return self.send_error(
                        span,
                        &uri,
                        format!("bad network_id: {:?}", network_id),
                    );
                }

                self.handle_received_remote_handshake(span, &uri, id)?;
            }
            InterimEncodingProtocol::Payload { payload } => {
                match self.connections_no_id_to_id.get(&uri) {
                    Some(remote_addr) => {
                        self.endpoint_self.publish(
                            span,
                            RequestToParent::ReceivedData {
                                uri: remote_addr.clone(),
                                payload,
                            },
                        )?;
                    }
                    None => {
                        // for some reason, the remote is sending us data
                        // without handshaking, let's try to handshake back?
                        self.send_handshake(span, &uri)?;

                        // store this msg to forward after we handshake
                        let e = self
                            .pending_received_data
                            .entry(uri)
                            .or_insert_with(|| vec![]);
                        e.push(payload);
                    }
                }
            }
        }
        Ok(())
    }

    /// private handler for inner transport TransportError events
    fn handle_transport_error(
        &mut self,
        span: Span,
        uri: Lib3hUri,
        error: TransportError,
    ) -> TransportResult<()> {
        // just forward this
        self.endpoint_self
            .publish(span, RequestToParent::ErrorOccured { uri, error })?;
        Ok(())
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
            RequestToChild::SendMessage { uri, payload } => {
                self.handle_send_message(msg, uri, payload)
            }
        }
    }

    /// private handler for Bind requests from our parent
    fn handle_bind(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        mut spec: Lib3hUri,
    ) -> TransportResult<()> {
        // remove any agent id from the spec
        // i.e. wss://1.2.3.4:55888?a=HcMyada -> wss://1.2.3.4:55888
        spec.set_query(None);

        // forward the bind to our inner_transport
        self.inner_transport.as_mut().request(
            msg.span().follower("TODO follower from message"),
            RequestToChild::Bind { spec },
            Box::new(|m: &mut TransportEncoding, response| {
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
                    // we got the bind result, append our id to it
                    // i.e. wss://1_bound?a=HcMyadyada
                    data.bound_url
                        .query_pairs_mut()
                        .append_pair("a", &m.this_id);
                    info!("got bind response: {:?}", data.bound_url);
                    msg.respond(Ok(RequestToChildResponse::Bind(data)))?;
                } else {
                    panic!("bad response to bind: {:?}", response);
                }
                Ok(())
            }),
        )?;
        Ok(())
    }

    /// handshake complete, or established connection
    /// forward SendMessage payload to our child && respond appropriately to
    /// our parent
    fn fwd_send_message_result(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        uri: Lib3hUri,
        payload: Opaque,
    ) -> TransportResult<()> {
        let payload = InterimEncodingProtocol::Payload { payload }.to_vec();
        self.inner_transport.as_mut().request(
            msg.span().follower("TODO follower from message"),
            RequestToChild::SendMessage { uri, payload },
            Box::new(|_: &mut TransportEncoding, response| {
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => response,
                    }
                };
                msg.respond(response)?;
                Ok(())
            }),
        )?;
        Ok(())
    }

    /// private handler for SendMessage requests from our parent
    fn handle_send_message(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        uri: Lib3hUri,
        payload: Opaque,
    ) -> TransportResult<()> {
        match self.connections_id_to_no_id.get(&uri) {
            Some(sub_address) => {
                // we have seen this connection before
                // we can just forward the message along
                let sub_address = sub_address.clone();
                self.fwd_send_message_result(msg, sub_address, payload)?;
            }
            None => {
                // we don't have an established connection to this remote
                // we need to handshake first

                // first make a low-level uri by removing the ?a=Hcyada
                let mut sub_address = uri.clone();
                sub_address.set_query(None);

                // send along a handshake message
                self.send_handshake(msg.span().follower("TODO follow from msg"), &sub_address)?;

                // store this send_data so we can forward it after handshake
                // (see handle_received_data for where this is done)
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
    fn take_parent_endpoint(&mut self) -> Option<TransportActorParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        detach_run!(&mut self.endpoint_self, |es| es.process(self))?;
        for msg in self.endpoint_self.as_mut().drain_messages() {
            self.handle_msg_from_parent(msg)?;
        }
        detach_run!(&mut self.inner_transport, |it| it.process(self))?;
        for msg in self.inner_transport.as_mut().drain_messages() {
            self.handle_msg_from_inner(msg)?;
        }
        detach_run!(&mut self.keystore, |ks| ks.process(self))?;
        Ok(false.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use holochain_tracing::test_span;
    use lib3h_sodium::SodiumCryptoSystem;
    use url::Url;

    const N_ID: &'static str = "HcNCjjt9OTf7Fk3wc3qH3bHsEKkcwb7gzY8SudxIvWxedi4iThtj4jE9fn8cpvz";
    const ID_1: &'static str = "HcSCJ9G64XDKYo433rIMm57wfI8Y59Udeb4hkVvQBZdm6bgbJ5Wgs79pBGBcuzz";
    const ID_2: &'static str = "HcMCJ8HpYvB4zqic93d3R4DjkVQ4hhbbv9UrZmWXOcn3m7w4O3AIr56JRfrt96r";

    pub struct TransportMock {
        endpoint_parent: Option<TransportActorParentEndpoint>,
        endpoint_self: Detach<
            GhostContextEndpoint<
                TransportMock,
                RequestToParent,
                RequestToParentResponse,
                RequestToChild,
                RequestToChildResponse,
                TransportError,
            >,
        >,
        bound_url: Lib3hUri,
        mock_sender: crossbeam_channel::Sender<(Lib3hUri, Opaque)>,
        mock_receiver: crossbeam_channel::Receiver<(Lib3hUri, Opaque)>,
    }

    impl TransportMock {
        pub fn new(
            mock_sender: crossbeam_channel::Sender<(Lib3hUri, Opaque)>,
            mock_receiver: crossbeam_channel::Receiver<(Lib3hUri, Opaque)>,
        ) -> Self {
            let (endpoint_parent, endpoint_self) = create_ghost_channel();
            let endpoint_parent = Some(endpoint_parent);
            let endpoint_self = Detach::new(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix("mock_to_parent_")
                    .build(),
            );
            Self {
                endpoint_parent,
                endpoint_self,
                bound_url: Lib3hUri::with_undefined(""),
                mock_sender,
                mock_receiver,
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
        fn take_parent_endpoint(&mut self) -> Option<TransportActorParentEndpoint> {
            std::mem::replace(&mut self.endpoint_parent, None)
        }

        fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
            detach_run!(&mut self.endpoint_self, |es| es.process(self))?;
            for mut msg in self.endpoint_self.as_mut().drain_messages() {
                match msg.take_message().expect("exists") {
                    RequestToChild::Bind { mut spec } => {
                        spec.set_path("bound");
                        self.bound_url = spec.clone();
                        msg.respond(Ok(RequestToChildResponse::Bind(BindResultData {
                            bound_url: spec,
                        })))?;
                    }
                    RequestToChild::SendMessage { uri, payload } => {
                        self.mock_sender.send((uri, payload)).unwrap();
                        msg.respond(Ok(RequestToChildResponse::SendMessageSuccess))?;
                    }
                }
            }
            loop {
                match self.mock_receiver.try_recv() {
                    Ok((uri, payload)) => {
                        // bit of a hack, just always send an incoming connection
                        // in front of all received data messages
                        self.endpoint_self.publish(
                            Span::fixme(),
                            RequestToParent::IncomingConnection { uri: uri.clone() },
                        )?;
                        self.endpoint_self.publish(
                            Span::fixme(),
                            RequestToParent::ReceivedData { uri, payload },
                        )?;
                    }
                    Err(_) => break,
                }
            }
            Ok(false.into())
        }
    }

    #[test]
    fn it_should_exchange_messages() {
        let crypto: Box<dyn CryptoSystem> =
            Box::new(SodiumCryptoSystem::new().set_pwhash_interactive());

        // set up some reference values
        let addr1: Lib3hUri = Url::parse("test://1/bound").unwrap().into();
        let addr2: Lib3hUri = Url::parse("test://2/bound").unwrap().into();
        let mut addr1full = addr1.clone();
        addr1full.query_pairs_mut().append_pair("a", ID_1);
        let mut addr2full = addr2.clone();
        addr2full.query_pairs_mut().append_pair("a", ID_2);

        // we need some channels into our mock inner_transports
        let (s1out, r1out) = crossbeam_channel::unbounded();
        let (s1in, r1in) = crossbeam_channel::unbounded();

        // create the first encoding transport
        let mut t1: TransportActorParentWrapper<bool, TransportEncoding> = GhostParentWrapper::new(
            TransportEncoding::new(
                crypto.box_clone(),
                ID_1.to_string(),
                N_ID.to_string(),
                Box::new(KeystoreStub::new()),
                Box::new(TransportMock::new(s1out, r1in)),
            ),
            "test1",
        );

        // give it a bind point
        t1.request(
            test_span(""),
            RequestToChild::Bind {
                spec: Url::parse("test://1").expect("can parse url").into(),
            },
            Box::new(|_: &mut bool, response| {
                assert_eq!(
                    &format!("{:?}", response),
                    "Response(Ok(Bind(BindResultData { bound_url: Lib3hUri(\"test://1/bound?a=HcSCJ9G64XDKYo433rIMm57wfI8Y59Udeb4hkVvQBZdm6bgbJ5Wgs79pBGBcuzz\") })))"
                );
                Ok(())
            })
        ).unwrap();

        // allow process
        t1.process(&mut false).unwrap();

        // we need some channels into our mock inner_transports
        let (s2out, r2out) = crossbeam_channel::unbounded();
        let (s2in, r2in) = crossbeam_channel::unbounded();

        // create the second encoding transport
        let mut t2: TransportActorParentWrapper<(), TransportEncoding> = GhostParentWrapper::new(
            TransportEncoding::new(
                crypto.box_clone(),
                ID_2.to_string(),
                N_ID.to_string(),
                Box::new(KeystoreStub::new()),
                Box::new(TransportMock::new(s2out, r2in)),
            ),
            "test2",
        );

        // give it a bind point
        t2.request(
            test_span(""),
            RequestToChild::Bind {
                spec: Url::parse("test://2").expect("can parse url").into(),
            },
            Box::new(|_:&mut (), response| {
                assert_eq!(
                    &format!("{:?}", response),
                    "Response(Ok(Bind(BindResultData { bound_url: Lib3hUri(\"test://2/bound?a=HcMCJ8HpYvB4zqic93d3R4DjkVQ4hhbbv9UrZmWXOcn3m7w4O3AIr56JRfrt96r\") })))"
                );
                Ok(())
            })
        ).unwrap();

        // allow process
        t2.process(&mut ()).unwrap();

        let mut t1_got_success_resp = false;

        // now we're going to send a message to our sibling #2
        t1.request(
            test_span(""),
            RequestToChild::SendMessage {
                uri: addr2full.clone(),
                payload: "hello".into(),
            },
            Box::new(|b: &mut bool, response| {
                *b = true;
                // make sure we get a success response
                assert_eq!(
                    "Response(Ok(SendMessageSuccess))",
                    format!("{:?}", response),
                );
                Ok(())
            }),
        )
        .unwrap();

        t1.process(&mut t1_got_success_resp).unwrap();

        // we get a handshake that needs to be forwarded to #2
        let (address, payload) = r1out.recv().unwrap();
        assert_eq!(addr2, address);
        assert_eq!(
            &Opaque::from(
                InterimEncodingProtocol::Handshake {
                    magic: 0x1f6c,
                    network_id: N_ID.to_string(),
                    id: ID_1.to_string(),
                }
                .to_vec()
            ),
            &payload,
        );
        s2in.send((addr1.clone().into(), payload)).unwrap();

        t2.process(&mut ()).unwrap();
        t1.process(&mut t1_got_success_resp).unwrap();
        t2.process(&mut ()).unwrap();

        // we get a handshake that needs to be forwarded to #1
        let (address, payload) = r2out.recv().unwrap();
        assert_eq!(addr1, address);
        assert_eq!(
            &Opaque::from(
                InterimEncodingProtocol::Handshake {
                    magic: 0x1f6c,
                    network_id: N_ID.to_string(),
                    id: ID_2.to_string(),
                }
                .to_vec()
            ),
            &payload,
        );
        s1in.send((addr2.clone().into(), payload)).unwrap();

        t1.process(&mut t1_got_success_resp).unwrap();
        t2.process(&mut ()).unwrap();

        // this is the process where we get the Send Success
        assert!(!t1_got_success_resp);
        t1.process(&mut t1_got_success_resp).unwrap();
        assert!(t1_got_success_resp);

        // this is another handshake due to our mock kludge
        r1out.recv().unwrap();

        t2.process(&mut ()).unwrap();

        // we get the actual payload that needs to be forwarded to #2
        let (address, payload) = r1out.recv().unwrap();
        assert_eq!(addr2, address);
        let expected: Opaque = InterimEncodingProtocol::Payload {
            payload: "hello".into(),
        }
        .to_vec()
        .into();
        assert_eq!(&expected, &payload);
        s2in.send((addr1.into(), payload)).unwrap();

        t2.process(&mut ()).unwrap();

        // #2 now gets the payload!
        let mut msg_list = t2.drain_messages();
        let msg = msg_list.get_mut(2).unwrap().take_message();
        if let Some(RequestToParent::ReceivedData { uri, payload }) = msg {
            assert_eq!(uri, addr1full);
            assert_eq!(&Opaque::from(b"hello".to_vec()), &payload);
        } else {
            panic!("bad type {:?}", msg);
        }
    }
}

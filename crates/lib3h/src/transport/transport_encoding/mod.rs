use std::any::Any;
use url::Url;
use detach::prelude::*;
use lib3h_crypto_api::{Buffer, CryptoSystem};
use lib3h_ghost_actor::prelude::*;
use crate::{
    keystore::*,
    transport::{
        error::*,
        protocol::*,
    }
};

enum ToParentContext {
}

enum ToInnerContext {
    AwaitBind(GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>),
}

enum ToKeystoreContext {
}

pub struct TransportEncoding {
    crypto: Box<dyn CryptoSystem>,
    this_id: String,
    keystore: Detach<KeystoreActorParentWrapper<ToKeystoreContext>>,
    endpoint_parent: Option<
        GhostEndpoint<
            RequestToChild,
            RequestToChildResponse,
            RequestToParent,
            RequestToParentResponse,
            TransportError,
        >,
    >,
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
    inner_transport: Detach<
        GhostParentWrapper<
            ToInnerContext,
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            TransportError,
        >,
    >,
}

impl TransportEncoding {
    pub fn new(crypto: Box<dyn CryptoSystem>, this_id: String, keystore: KeystoreActor, inner_transport: Box<dyn GhostActor<RequestToParent, RequestToParentResponse, RequestToChild, RequestToChildResponse, TransportError>>) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(endpoint_self.as_context_endpoint("enc_to_parent_"));
        let keystore = Detach::new(GhostParentWrapper::new(keystore, "enc_to_keystore"));
        let inner_transport = Detach::new(GhostParentWrapper::new(inner_transport, "enc_to_inner_"));
        Self {
            crypto,
            this_id,
            keystore,
            endpoint_parent,
            endpoint_self,
            inner_transport,
        }
    }

    fn handle_msg_from_inner(&mut self, mut msg: GhostMessage<RequestToParent, RequestToChild, RequestToParentResponse, TransportError>) -> TransportResult<()> {
        match msg.take_message().expect("exists") {
            RequestToParent::IncomingConnection { address } => self.handle_incoming_connection(address),
            RequestToParent::ReceivedData { address, payload } => self.handle_received_data(address, payload),
            RequestToParent::TransportError { error } => self.handle_transport_error(error),
        }
    }

    fn handle_incoming_connection(&mut self, mut address: Url) -> TransportResult<()> {
        let remote_addr = "badwrong"; // TODO XXX - handshake remote id
        address.query_pairs_mut().append_pair("a", remote_addr);
        self.endpoint_self.publish(RequestToParent::IncomingConnection {
            address,
        });
        Ok(())
    }

    fn handle_received_data(&mut self, mut address: Url, payload: Vec<u8>) -> TransportResult<()> {
        let remote_addr = "badwrong"; // TODO XXX - handshake remote id
        address.query_pairs_mut().append_pair("a", remote_addr);
        self.endpoint_self.publish(RequestToParent::ReceivedData {
            address, payload
        });
        Ok(())
    }

    fn handle_transport_error(&mut self, error: TransportError) -> TransportResult<()> {
        self.endpoint_self.publish(RequestToParent::TransportError {
            error,
        });
        Ok(())
    }

    fn handle_msg_from_parent(&mut self, mut msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>) -> TransportResult<()> {
        match msg.take_message().expect("exists") {
            RequestToChild::Bind { spec } => self.handle_bind(msg, spec),
            RequestToChild::SendMessage { address, payload } => self.handle_send_message(msg, address, payload),
        }
    }

    fn handle_bind(&mut self, mut msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>, spec: Url) -> TransportResult<()> {
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
                    }
                };
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => {
                            match response {
                                Err(e) => panic!("{:?}", e),
                                Ok(response) => response,
                            }
                        }
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
            })
        );
        Ok(())
    }

    fn handle_send_message(&mut self, mut msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>, address: Url, payload: Vec<u8>) -> TransportResult<()> {
        unimplemented!()
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

    fn take_parent_endpoint(
        &mut self
    ) -> Option<
        GhostEndpoint<
            RequestToChild,
            RequestToChildResponse,
            RequestToParent,
            RequestToParentResponse,
            TransportError,
        >,
    > {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        detach_run!(&mut self.endpoint_self, |es| es.process(self.as_any()))?;
        for mut msg in self.endpoint_self.as_mut().drain_messages() {
            self.handle_msg_from_parent(msg)?;
        }
        detach_run!(&mut self.inner_transport, |it| it.process(self.as_any()))?;
        for mut msg in self.inner_transport.as_mut().drain_messages() {
            self.handle_msg_from_inner(msg)?;
        }
        Ok(false.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib3h_sodium::SodiumCryptoSystem;

    const ID_1: &'static str = "HcSCJ9G64XDKYo433rIMm57wfI8Y59Udeb4hkVvQBZdm6bgbJ5Wgs79pBGBcuzz";
    // const ID_2: &'static str = "HcMCJ8HpYvB4zqic93d3R4DjkVQ4hhbbv9UrZmWXOcn3m7w4O3AIr56JRfrt96r";

    pub struct TransportMock {
        endpoint_parent: Option<
            GhostEndpoint<
                RequestToChild,
                RequestToChildResponse,
                RequestToParent,
                RequestToParentResponse,
                TransportError,
            >,
        >,
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
            let endpoint_self = Detach::new(endpoint_self.as_context_endpoint("transport_encoding_to_parent_"));
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

        fn take_parent_endpoint(
            &mut self
        ) -> Option<
            GhostEndpoint<
                RequestToChild,
                RequestToChildResponse,
                RequestToParent,
                RequestToParentResponse,
                TransportError,
            >,
        > {
            std::mem::replace(&mut self.endpoint_parent, None)
        }

        fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
            Ok(false.into())
        }
    }


    #[test]
    fn it_should_create_two() {
        let crypto: Box<dyn CryptoSystem> =
            Box::new(SodiumCryptoSystem::new().set_pwhash_interactive());

        let m1 = Box::new(TransportMock::new());
        let t1 = TransportEncoding::new(
            crypto.box_clone(),
            ID_1.to_string().into(),
            Box::new(KeystoreStub::new()),
            m1
        );
    }
}

use std::any::Any;
use url::Url;
use detach::prelude::*;
use lib3h_protocol::Address;
use lib3h_crypto_api::{Buffer, CryptoSystem};
use lib3h_ghost_actor::prelude::*;
use crate::transport::{
    error::*,
    protocol::*,
};

enum ToParentContext {
}

enum ToInnerContext {
}

pub struct TransportEncoding {
    crypto: Box<dyn CryptoSystem>,
    this_id: Address,
    this_sig_secret: Box<dyn Buffer>,
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
    pub fn new(crypto: Box<dyn CryptoSystem>, this_id: Address, this_sig_secret: Box<dyn Buffer>, inner_transport: Box<dyn GhostActor<RequestToParent, RequestToParentResponse, RequestToChild, RequestToChildResponse, TransportError>>) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(endpoint_self.as_context_endpoint("transport_encoding_to_parent_"));
        let inner_transport = Detach::new(GhostParentWrapper::new(inner_transport, "transport_encoding_to_inner_"));
        Self {
            crypto,
            this_id,
            this_sig_secret,
            endpoint_parent,
            endpoint_self,
            inner_transport,
        }
    }

    fn handle_msg_from_parent(&mut self, mut msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>) -> TransportResult<()> {
        match msg.take_message().expect("exists") {
            RequestToChild::Bind { spec } => self.handle_bind(msg, spec),
            RequestToChild::SendMessage { address, payload } => self.handle_send_message(msg, address, payload),
        }
    }

    fn handle_bind(&mut self, mut msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>, spec: Url) -> TransportResult<()> {
        unimplemented!()
    }

    fn handle_send_message(&mut self, mut msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>, address: Url, payload: Vec<u8>) -> TransportResult<()> {
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
        Ok(false.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib3h_sodium::SodiumCryptoSystem;

    const ID_1: &'static str = "HcSCJ9G64XDKYo433rIMm57wfI8Y59Udeb4hkVvQBZdm6bgbJ5Wgs79pBGBcuzz";
    const KEY_1: &'static [u8] = &[66, 112, 30, 162, 214, 166, 57, 123, 140, 60, 53, 143, 4, 145, 3, 202, 47, 130, 231, 130, 62, 235, 248, 182, 217, 24, 9, 40, 113, 13, 89, 33, 252, 220, 213, 134, 171, 183, 89, 204, 16, 181, 239, 181, 42, 61, 125, 254, 99, 32, 116, 117, 82, 143, 14, 6, 190, 4, 193, 78, 234, 104, 247, 238];
    /*
    const ID_2: &'static str = "HcMCJ8HpYvB4zqic93d3R4DjkVQ4hhbbv9UrZmWXOcn3m7w4O3AIr56JRfrt96r";
    const KEY_2: &'static [u8] = &[223, 124, 226, 203, 1, 71, 58, 183, 19, 146, 36, 202, 168, 208, 89, 195, 17, 111, 233, 110, 68, 31, 3, 147, 43, 47, 23, 26, 192, 130, 135, 178, 248, 238, 189, 3, 172, 61, 2, 254, 71, 152, 104, 105, 85, 31, 163, 156, 33, 167, 231, 12, 46, 182, 104, 153, 149, 246, 186, 110, 64, 136, 111, 137];
    */

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

        let mut key1 = crypto.buf_new_secure(crypto.sign_secret_key_bytes());
        key1.write(0, KEY_1).unwrap();

        let m1 = Box::new(TransportMock::new());
        let t1 = TransportEncoding::new(
            crypto.box_clone(), ID_1.to_string().into(), key1, m1);
    }
}

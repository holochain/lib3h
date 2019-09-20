//! utility actor for encoding / decoding messages

use crate::error::{Lib3hError, Lib3hResult};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::data_types::Opaque;

/// temporary protocol enum for wire encoding
#[derive(Debug, Clone, Serialize, Deserialize)]
enum InterimEncodingProtocol {
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
    fn to_opaque(&self) -> Opaque {
        serde_json::to_vec_pretty(self).unwrap().into()
    }

    fn from_slice(v: &[u8]) -> Self {
        match serde_json::from_slice(v) {
            Ok(s) => s,
            Err(e) => panic!(
                "failed to decode {:?} - {:?}",
                String::from_utf8_lossy(v),
                e
            ),
        }
    }
}

pub mod encoding_protocol {
    use super::*;

    #[derive(Debug)]
    pub enum RequestToChild {
        Decode { payload: Opaque },
        EncodeHandshake { space_address: String, id: String },
        EncodePayload { payload: Opaque },
    }

    #[derive(Debug)]
    pub enum DecodeData {
        Handshake { space_address: String, id: String },
        Payload { payload: Opaque },
    }

    #[derive(Debug)]
    pub enum RequestToChildResponse {
        DecodeResult { result: DecodeData },
        EncodeHandshakeResult { payload: Opaque },
        EncodePayloadResult { payload: Opaque },
    }

    #[derive(Debug)]
    pub enum RequestToParent {}

    #[derive(Debug)]
    pub enum RequestToParentResponse {}
}

use encoding_protocol::*;

pub type MessageEncodingActorParentEndpoint = GhostEndpoint<
    RequestToChild,
    RequestToChildResponse,
    RequestToParent,
    RequestToParentResponse,
    Lib3hError,
>;

pub type MessageEncodingActorParentWrapper<T> = GhostParentWrapper<
    T,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    Lib3hError,
    MessageEncoding,
>;

type MessageEncodingParentEndpoint = GhostEndpoint<
    RequestToChild,
    RequestToChildResponse,
    RequestToParent,
    RequestToParentResponse,
    Lib3hError,
>;

type MessageEncodingSelfEndpoint = GhostContextEndpoint<
    MessageEncoding,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    Lib3hError,
>;

type MessageEncodingMessageFromParent =
    GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, Lib3hError>;

pub struct MessageEncoding {
    endpoint_parent: Option<MessageEncodingParentEndpoint>,
    endpoint_self: Detach<MessageEncodingSelfEndpoint>,
}

impl MessageEncoding {
    pub fn new() -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(
            endpoint_self
                .as_context_endpoint_builder()
                .request_id_prefix("encoding_to_parent_")
                .build(),
        );
        Self {
            endpoint_parent,
            endpoint_self,
        }
    }

    fn handle_msg_from_parent(
        &mut self,
        mut msg: MessageEncodingMessageFromParent,
    ) -> Lib3hResult<()> {
        error!("{:?}", msg.backtrace());
        match msg.take_message().expect("exists") {
            RequestToChild::Decode { payload } => self.handle_decode(msg, payload),
            RequestToChild::EncodeHandshake { space_address, id } => {
                self.handle_encode_handshake(msg, space_address, id)
            }
            RequestToChild::EncodePayload { payload } => self.handle_encode_payload(msg, payload),
        }
    }

    fn handle_decode(
        &mut self,
        msg: MessageEncodingMessageFromParent,
        payload: Opaque,
    ) -> Lib3hResult<()> {
        let payload = match InterimEncodingProtocol::from_slice(&payload) {
            InterimEncodingProtocol::Handshake {
                magic,
                network_id,
                id,
            } => {
                if magic != 0x1f6c {
                    msg.respond(Err("bad magic".into()))?;
                    return Ok(());
                }
                DecodeData::Handshake {
                    space_address: network_id,
                    id,
                }
            }
            InterimEncodingProtocol::Payload { payload } => DecodeData::Payload { payload },
        };
        msg.respond(Ok(RequestToChildResponse::DecodeResult { result: payload }))?;
        Ok(())
    }

    fn handle_encode_handshake(
        &mut self,
        msg: MessageEncodingMessageFromParent,
        space_address: String,
        id: String,
    ) -> Lib3hResult<()> {
        let payload = InterimEncodingProtocol::Handshake {
            magic: 0x1f6c,
            network_id: space_address,
            id: id,
        }
        .to_opaque();
        msg.respond(Ok(RequestToChildResponse::EncodeHandshakeResult {
            payload: payload,
        }))?;
        Ok(())
    }

    fn handle_encode_payload(
        &mut self,
        msg: MessageEncodingMessageFromParent,
        payload: Opaque,
    ) -> Lib3hResult<()> {
        let payload = InterimEncodingProtocol::Payload { payload }.to_opaque();
        msg.respond(Ok(RequestToChildResponse::EncodePayloadResult { payload }))?;
        Ok(())
    }
}

impl
    GhostActor<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Lib3hError,
    > for MessageEncoding
{
    fn take_parent_endpoint(&mut self) -> Option<MessageEncodingParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        detach_run!(&mut self.endpoint_self, |es| es.process(self))?;
        for msg in self.endpoint_self.as_mut().drain_messages() {
            self.handle_msg_from_parent(msg).expect("no ghost errors");
        }
        Ok(false.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_handshake() {
        let mut e: MessageEncodingActorParentWrapper<String> =
            GhostParentWrapper::new(MessageEncoding::new(), "test");

        let mut in_out = "".to_string();

        e.request(
            holochain_tracing::test_span(""),
            RequestToChild::EncodeHandshake {
                space_address: "space-1".to_string(),
                id: "id-1".to_string(),
            },
            Box::new(|out: &mut String, resp| {
                out.clear();
                match resp {
                    GhostCallbackData::Response(Ok(
                        RequestToChildResponse::EncodeHandshakeResult { payload },
                    )) => {
                        out.push_str(&String::from_utf8_lossy(&payload));
                    }
                    _ => panic!("bad type: {:?}", resp),
                }
                Ok(())
            }),
        )
        .unwrap();

        e.process(&mut in_out).unwrap();

        assert_eq!("{\n  \"Handshake\": {\n    \"magic\": 8044,\n    \"network_id\": \"space-1\",\n    \"id\": \"id-1\"\n  }\n}", &in_out);

        e.request(
            holochain_tracing::test_span(""),
            RequestToChild::Decode {
                payload: in_out.as_bytes().into(),
            },
            Box::new(|out: &mut String, resp| {
                out.clear();
                match resp {
                    GhostCallbackData::Response(Ok(RequestToChildResponse::DecodeResult {
                        result: DecodeData::Handshake { space_address, id },
                    })) => {
                        out.push_str(&format!("{} {}", space_address, id));
                    }
                    _ => panic!("bad type: {:?}", resp),
                }
                Ok(())
            }),
        )
        .unwrap();

        e.process(&mut in_out).unwrap();

        assert_eq!("space-1 id-1", &in_out);
    }

    #[test]
    fn it_should_payload() {
        let mut e: MessageEncodingActorParentWrapper<String> =
            GhostParentWrapper::new(MessageEncoding::new(), "test");

        let mut in_out = "".to_string();

        e.request(
            holochain_tracing::test_span(""),
            RequestToChild::EncodePayload {
                payload: b"test".to_vec().into(),
            },
            Box::new(|out: &mut String, resp| {
                out.clear();
                match resp {
                    GhostCallbackData::Response(Ok(
                        RequestToChildResponse::EncodePayloadResult { payload },
                    )) => {
                        out.push_str(&String::from_utf8_lossy(&payload));
                    }
                    _ => panic!("bad type: {:?}", resp),
                }
                Ok(())
            }),
        )
        .unwrap();

        e.process(&mut in_out).unwrap();

        assert_eq!(
            "{\n  \"Payload\": {\n    \"payload\": \"dGVzdA==\"\n  }\n}",
            &in_out
        );

        e.request(
            holochain_tracing::test_span(""),
            RequestToChild::Decode {
                payload: in_out.as_bytes().into(),
            },
            Box::new(|out: &mut String, resp| {
                out.clear();
                match resp {
                    GhostCallbackData::Response(Ok(RequestToChildResponse::DecodeResult {
                        result: DecodeData::Payload { payload },
                    })) => {
                        out.push_str(&format!("{:?}", payload));
                    }
                    _ => panic!("bad type: {:?}", resp),
                }
                Ok(())
            }),
        )
        .unwrap();

        e.process(&mut in_out).unwrap();

        assert_eq!("\"test\"", &in_out);
    }
}

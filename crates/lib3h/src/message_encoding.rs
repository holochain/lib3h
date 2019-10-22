//! utility actor for encoding / decoding messages

use crate::{
    error::{Lib3hError, Lib3hResult},
    keystore2::*,
};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::Opaque, types::NetworkHash, Address};

const CURRENT_ENCODING_HEURISTIC_MAGIC: u16 = 0x1f6c;

/// temporary protocol enum for wire encoding
#[derive(Debug, Clone, Serialize, Deserialize)]
enum InterimEncodingProtocol {
    Handshake {
        magic: u16,
        network_id: NetworkHash,
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
        Decode {
            payload: Opaque,
        },
        EncodeHandshake {
            network_or_space_address: Address,
            id: String,
        },
        EncodePayload {
            payload: Opaque,
        },
    }

    #[derive(Debug)]
    pub enum DecodeData {
        Handshake {
            network_or_space_address: Address,
            id: String,
        },
        Payload {
            payload: Opaque,
        },
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

pub type MessageEncodingActorParentWrapper<'lt, T> = GhostParentWrapper<
    T,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    Lib3hError,
    MessageEncoding<'lt>,
>;

type MessageEncodingParentEndpoint = GhostEndpoint<
    RequestToChild,
    RequestToChildResponse,
    RequestToParent,
    RequestToParentResponse,
    Lib3hError,
>;

type MessageEncodingSelfEndpoint<'lt> = GhostContextEndpoint<
    MessageEncoding<'lt>,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    Lib3hError,
>;

type MessageEncodingMessageFromParent =
    GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, Lib3hError>;

pub struct MessageEncoding<'lt> {
    endpoint_parent: Option<MessageEncodingParentEndpoint>,
    endpoint_self: Detach<MessageEncodingSelfEndpoint<'lt>>,
    local_system: ghost_actor::SingleThreadedGhostSystem<'lt>,
    #[allow(dead_code)]
    ghost_context: std::sync::Arc<ghost_actor::GhostMutex<()>>,
    keystore_ref: ghost_actor::GhostEndpointFull<
        'lt,
        keystore_protocol::KeystoreProtocol,
        Keystore2ActorStub<'lt, ghost_actor::SingleThreadedGhostSystemRef<'lt>>,
        (),
        keystore_protocol::KeystoreOwnerHandler<'lt, ()>,
        ghost_actor::SingleThreadedGhostSystemRef<'lt>,
    >,
}

impl<'lt> MessageEncoding<'lt> {
    pub fn new() -> Self {
        // stub keystore for now
        // also micro/local ghost system for now
        use ghost_actor::GhostSystem;
        let local_system = ghost_actor::SingleThreadedGhostSystem::new();
        let ghost_context = std::sync::Arc::new(ghost_actor::GhostMutex::new(()));
        let (mut sys_ref, finalize) = local_system.create_external_system_ref();
        finalize(std::sync::Arc::downgrade(&ghost_context)).expect("can finalize ghost system");

        let keystore_ref = sys_ref
            .spawn(
                Box::new(|sys_ref, owner_seed| Keystore2ActorStub::new(sys_ref, owner_seed)),
                keystore_protocol::KeystoreOwnerHandler {
                    phantom: std::marker::PhantomData,
                },
            )
            .expect("can spawn stub keystore actor");

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
            local_system,
            ghost_context,
            keystore_ref,
        }
    }

    fn handle_msg_from_parent(
        &mut self,
        mut msg: MessageEncodingMessageFromParent,
    ) -> Lib3hResult<()> {
        match msg.take_message().expect("exists") {
            RequestToChild::Decode { payload } => self.handle_decode(msg, payload),
            RequestToChild::EncodeHandshake {
                network_or_space_address,
                id,
            } => self.handle_encode_handshake(msg, network_or_space_address.into(), id),
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
                if magic != CURRENT_ENCODING_HEURISTIC_MAGIC {
                    msg.respond(Err(format!(
                        "bad magic, cannot speak to remote peer {} {}",
                        network_id, id
                    )
                    .into()))?;
                    return Ok(());
                }
                DecodeData::Handshake {
                    network_or_space_address: network_id.into(),
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
        network_id: NetworkHash,
        id: String,
    ) -> Lib3hResult<()> {
        let payload = InterimEncodingProtocol::Handshake {
            magic: CURRENT_ENCODING_HEURISTIC_MAGIC,
            network_id,
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
        // fake doing a signature of the payload
        use keystore_protocol::KeystoreActorRef;
        self.keystore_ref.request_to_actor_sign(
            None,
            keystore_protocol::SignRequestData {
                pub_key: keystore_protocol::PubKey::AgentPubKey("fake".into()),
                data: payload.clone(),
            },
            Box::new(move |_span, _me, _signature| {
                msg.respond(Ok(RequestToChildResponse::EncodePayloadResult { payload }))?;
                Ok(())
            }),
        )?;
        Ok(())
    }
}

impl<'lt>
    GhostActor<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Lib3hError,
    > for MessageEncoding<'lt>
{
    fn take_parent_endpoint(&mut self) -> Option<MessageEncodingParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // process our local ghost_actor 2.0 system
        use ghost_actor::GhostSystem;
        self.local_system.process()?;

        let mut did_work = detach_run!(&mut self.endpoint_self, |es| es.process(self))?;

        for msg in self.endpoint_self.as_mut().drain_messages() {
            did_work = true.into();
            self.handle_msg_from_parent(msg).expect("no ghost errors");
        }

        Ok(did_work)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_handshake() {
        let mut e: MessageEncodingActorParentWrapper<String> =
            GhostParentWrapper::new(MessageEncoding::new(), "test");

        let mut in_out = String::new();

        e.request(
            holochain_tracing::test_span(""),
            RequestToChild::EncodeHandshake {
                network_or_space_address: "space-1".into(),
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
                        result:
                            DecodeData::Handshake {
                                network_or_space_address,
                                id,
                            },
                    })) => {
                        out.push_str(&format!("{} {}", network_or_space_address, id));
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
        e.process(&mut in_out).unwrap();
        e.process(&mut in_out).unwrap();
        e.process(&mut in_out).unwrap();
        e.process(&mut in_out).unwrap();
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

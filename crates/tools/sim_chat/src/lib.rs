//! SimChat is a simulation test framework backend for lib3h
//! - See the integration tests for automated usage
//! - See half_busy_chat_cli for manual simulation

extern crate base64;
extern crate crossbeam_channel;
extern crate url;

use core::convert::{TryFrom, TryInto};
use lib3h::engine::ghost_engine::{
    GhostEngine,
    EngineError,
};
use lib3h_crypto_api::CryptoError;
use lib3h_ghost_actor::{GhostActor, GhostCanTrack, GhostContextEndpoint};
use lib3h_protocol::{
    data_types::{SpaceData, ConnectData},
    protocol::{
        ClientToLib3h,
        ClientToLib3hResponse,
        Lib3hToClient,
        Lib3hToClientResponse,
    },
    Address,
};
use lib3h_sodium::{hash, secbuf::SecBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEvent {
    SendDirectMessage {
        to_address: String,
        payload: String,
    },
    ReceiveDirectMessage {
        from_address: String,
        payload: String,
    },
    Join(Address),
    Part(Address),
}

impl TryFrom<Lib3hToClient> for ChatEvent {
    type Error = String;
    fn try_from(lib3h_message: Lib3hToClient) -> Result<ChatEvent, Self::Error> {
        match lib3h_message {
            Lib3hToClient::HandleSendDirectMessage(message_data) => {
                Ok(ChatEvent::ReceiveDirectMessage {
                    from_address: message_data.from_agent_id.to_string(),
                    payload: "message from engine".to_string(),
                })
            }
            _ => Err("Nope".to_string()),
        }
    }
}

impl TryInto<ClientToLib3h> for ChatEvent {
    type Error = String;
    fn try_into(self) -> Result<ClientToLib3h, Self::Error> {
        match self {
            ChatEvent::Join(ref addr) => {
                let space_data = SpaceData {
                    agent_id: Address::new(),
                    request_id: "".to_string(),
                    space_address: addr.to_owned(),
                };
                Ok(ClientToLib3h::JoinSpace(space_data))
            }
            ChatEvent::Part(ref addr) => {
                let space_data = SpaceData {
                    agent_id: Address::new(),
                    request_id: "".to_string(),
                    space_address: addr.to_owned(),
                };
                Ok(ClientToLib3h::LeaveSpace(space_data))
            }
            _ => Err("Not implemented".to_string()),
        }
    }
}

pub type HandleEvent = Box<dyn FnMut(&ChatEvent) + Send>;

pub struct SimChat {
    thread: Option<std::thread::JoinHandle<()>>,
    thread_continue: Arc<AtomicBool>,
    out_send: crossbeam_channel::Sender<ChatEvent>,
}

impl Drop for SimChat {
    fn drop(&mut self) {
        self.thread_continue.store(false, Ordering::Relaxed);
        std::mem::replace(&mut self.thread, None)
            .expect("cannot drop, thread already None")
            .join()
            .expect("thead join failed");
    }
}

impl SimChat {
    pub fn new(mut handler: HandleEvent) -> Self {
        let thread_continue = Arc::new(AtomicBool::new(true));

        let (out_send, out_recv): (
            crossbeam_channel::Sender<_>,
            crossbeam_channel::Receiver<ChatEvent>,
        ) = crossbeam_channel::unbounded();

        let thread_continue_inner = thread_continue.clone();
        Self {
            thread: Some(std::thread::spawn(move || {
                // this thread owns the ghost engine instance
                // and is responsible for calling process
                // and handling messages
                let mut lib3h_engine = GhostEngine::new();
                let mut parent_endpoint: GhostContextEndpoint<(), String, _, _, _, _, _> =
                    lib3h_engine
                        .take_parent_endpoint()
                        .unwrap()
                        .as_context_endpoint_builder()
                        .request_id_prefix("parent")
                        .build();

                // call connect to start the networking process
                SimChat::connect(&mut parent_endpoint, Url::parse("http://bootstrap.holo.host").unwrap());

                while thread_continue_inner.load(Ordering::Relaxed) {
                    // call process to make stuff happen
                    parent_endpoint.process(&mut ()).unwrap();
                    lib3h_engine.process().unwrap();

                    // gather all the ChatEvents
                    // Receive directly from the crossbeam channel
                    // and convert relevent N3H protocol messages to chat events
                    let direct_chat_events = out_recv.try_iter();
                    let n3h_chat_events = parent_endpoint.drain_messages().into_iter().filter_map(
                        |mut n3h_message| {
                            ChatEvent::try_from(n3h_message.take_message().unwrap()).ok()
                        },
                    );

                    // process all the chat events by calling the handler for all events
                    // and dispatching new n3h actions where required
                    direct_chat_events
                        .chain(n3h_chat_events)
                        .for_each(|chat_event| {
                            handler(&chat_event);
                            chat_event
                                .try_into()
                                .and_then(|lib3h_message: ClientToLib3h| {
                                    parent_endpoint.request(
                                        String::from("ctx"),
                                        lib3h_message,
                                        Box::new(|_, _, callback_data| {
                                            println!(
                                                "chat received response from engine: {:?}",
                                                callback_data
                                            );
                                            Ok(())
                                        }),
                                    );
                                    Ok(())
                                })
                                .ok();
                        });

                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            })),
            thread_continue,
            out_send,
        }
    }

    pub fn send(&mut self, event: ChatEvent) {
        self.out_send.send(event).expect("send fail");
    }

    fn connect(
        endpoint: &mut GhostContextEndpoint<
            (),
            String,
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hToClient,
            Lib3hToClientResponse,
            EngineError>,
        peer_uri: Url,
    ) {
        let connect_message = ClientToLib3h::Connect(ConnectData{
            network_id: String::from(""), // connect to any
            peer_uri,
            request_id: String::from("connect-request"),
        });
        endpoint.request(
            String::from("ctx"),
            connect_message,
            Box::new(|_, _, callback_data| {
                println!(
                    "chat received response from engine: {:?}",
                    callback_data
                );
                Ok(())
            }),
        );
    }
}

pub fn channel_address_from_str(channel_id: &str) -> Result<Address, CryptoError> {
    let mut input = SecBuf::with_insecure_from_string(channel_id.to_string());
    let mut output = SecBuf::with_insecure(hash::BYTES256);
    hash::sha256(&mut input, &mut output).unwrap();
    let buf = output.read_lock();
    let signature_str = base64::encode(&**buf);
    Ok(Address::from(signature_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_echo() {
        let (s, r) = crossbeam_channel::unbounded();
        let mut chat = SimChat::new(Box::new(move |event| {
            s.send(event.to_owned()).expect("send fail");
        }));

        let msg = ChatEvent::SendDirectMessage {
            to_address: "addr".to_string(),
            payload: "yo".to_string(),
        };

        chat.send(msg.clone());

        assert_eq!(msg, r.recv().expect("receive fail"));
    }

    #[test]
    fn can_convert_strings_to_channel_address() {
        let addr = channel_address_from_str("test channel id").expect("failed to hash string");
        assert_eq!(
            addr,
            Address::from("mgb/+MzdPWAYRs4ERGlj3WCg53AddXsg1HjXH7pk7pE=".to_string())
        )
    }
}

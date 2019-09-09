//! SimChat is a simulation test framework backend for lib3h
//! - See the integration tests for automated usage
//! - See half_busy_chat_cli for manual simulation

extern crate base64;
extern crate crossbeam_channel;
extern crate url;

use core::convert::{TryFrom};
use lib3h::engine::ghost_engine::{
    GhostEngine,
    EngineError,
};
use lib3h_crypto_api::CryptoError;
use lib3h_ghost_actor::{GhostActor, GhostCanTrack, GhostContextEndpoint};
use lib3h_protocol::{
    data_types::{SpaceData, ConnectData, DirectMessageData},
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

#[derive(Debug, Clone, PartialEq)]
pub enum ChatEvent {
    SendDirectMessage {
        to_agent: String,
        payload: String,
    },
    ReceiveDirectMessage {
        from_agent: String,
        payload: String,
    },
    Join {
        channel_id: String,
        agent_id: String,
    },
    JoinSuccess {
        channel_id: String,
        space_data: SpaceData,
    },
    Part,
    PartSuccess,
    Disconnected,
}

impl TryFrom<Lib3hToClient> for ChatEvent {
    type Error = String;
    fn try_from(lib3h_message: Lib3hToClient) -> Result<ChatEvent, Self::Error> {
        match lib3h_message {
            Lib3hToClient::HandleSendDirectMessage(message_data) => {
                Ok(ChatEvent::ReceiveDirectMessage {
                    from_agent: message_data.from_agent_id.to_string(),
                    payload: "message from engine".to_string(),
                })
            },
            Lib3hToClient::Disconnected(_) => Ok(ChatEvent::Disconnected),
            _ => Err("Unknown Lib3h message".to_string()),
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
    pub fn new(mut handler: HandleEvent, peer_uri: Url) -> Self {
        let thread_continue = Arc::new(AtomicBool::new(true));

        let (out_send, out_recv): (
            crossbeam_channel::Sender<_>,
            crossbeam_channel::Receiver<ChatEvent>,
        ) = crossbeam_channel::unbounded();

        let internal_sender = out_send.clone();

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

                // also keep track of things like the current space in this scope
                let mut current_space: Option<SpaceData> = None;

                // call connect to start the networking process
                // (should probably wait for confirmatio before continuing)
                SimChat::connect(&mut parent_endpoint, peer_uri);

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
                    for chat_event in direct_chat_events.chain(n3h_chat_events) {

                        let local_internal_sender = internal_sender.clone();

                        // every chat event call the handler that was passed
                        handler(&chat_event);

                        // also do internal logic for certain events e.g. converting them to lib3h events
                        // and also handling the responses to mutate local state
                        match chat_event {

                            ChatEvent::Join{channel_id, agent_id} => {
                                let space_address = channel_address_from_string(&channel_id).unwrap();
                                let space_data = SpaceData {
                                    agent_id: Address::from(agent_id),
                                    request_id: "".to_string(),
                                    space_address,
                                };
                                parent_endpoint.request(
                                    String::from("ctx"),
                                    ClientToLib3h::JoinSpace(space_data.clone()),
                                    Box::new(move |_, _, _callback_data| {
                                        // TODO: check the response was actually a success
                                        local_internal_sender.send(ChatEvent::JoinSuccess {
                                            channel_id: channel_id.clone(),
                                            space_data: space_data.clone()
                                        }).unwrap();
                                        Ok(())
                                    }),
                                );
                            },

                            ChatEvent::JoinSuccess{space_data, channel_id} => {
                                current_space = Some(space_data);
                                SimChat::send_sys_message(local_internal_sender, &format!("Joined channel: {}", channel_id));
                            },

                            ChatEvent::Part => {
                                if let Some(space_data) = current_space.clone() {
                                    parent_endpoint.request(
                                        String::from("ctx"),
                                        ClientToLib3h::LeaveSpace(space_data.to_owned()),
                                        Box::new(move |_, _, _callback_data| {
                                            local_internal_sender.send(ChatEvent::PartSuccess).unwrap();
                                            Ok(())
                                        }),
                                    );
                                } else {
                                    SimChat::send_sys_message(local_internal_sender, &"No channel to leave".to_string());
                                }
                            },

                            ChatEvent::PartSuccess => {
                                current_space = None;
                                SimChat::send_sys_message(local_internal_sender, &"Left channel".to_string());
                            }

                            ChatEvent::SendDirectMessage{ to_agent, payload } => {
                                if let Some(space_data) = current_space.clone() {
                                    let direct_message_data = DirectMessageData {
                                        from_agent_id: space_data.agent_id,
                                        content: payload.into(),
                                        to_agent_id: Address::from(to_agent),
                                        space_address: space_data.space_address,
                                        request_id: String::from(""),
                                    };
                                    parent_endpoint.request(
                                        String::from("ctx"),
                                        ClientToLib3h::SendDirectMessage(direct_message_data),
                                        Box::new(|_, _, callback_data| {
                                            println!(
                                                "direct message request received response from engine: {:?}",
                                                callback_data
                                            );
                                            Ok(())
                                        }),
                                    );
                                } else {
                                    println!("Must join a channel before sending a message")
                                }
                            },

                            _ => {}
                        }
                    }

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

    fn send_sys_message(sender: crossbeam_channel::Sender<ChatEvent>, msg: &String) {
        sender.send(ChatEvent::ReceiveDirectMessage {
            from_agent: String::from("sys"),
            payload: String::from(msg)
        }).expect("send fail");
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

pub fn channel_address_from_string(channel_id: &String) -> Result<Address, CryptoError> {
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
        let mut chat = SimChat::new(
            Box::new(move |event| {
                s.send(event.to_owned()).expect("send fail");
            }),
            Url::parse("http://test.boostrap").unwrap()
        );

        let msg = ChatEvent::SendDirectMessage {
            to_agent: "addr".to_string(),
            payload: "yo".to_string(),
        };

        chat.send(msg.clone());

        assert_eq!(msg, r.recv().expect("receive fail"));
    }

    #[test]
    fn can_convert_strings_to_channel_address() {
        let addr = channel_address_from_string(&String::from("test channel id")).expect("failed to hash string");
        assert_eq!(
            addr,
            Address::from("mgb/+MzdPWAYRs4ERGlj3WCg53AddXsg1HjXH7pk7pE=".to_string())
        )
    }
}

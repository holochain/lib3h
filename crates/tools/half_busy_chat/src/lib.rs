//! HalfBusyChat is a simulation test framework backend for lib3h
//! - See the integration tests for automated usage
//! - See half_busy_chat_cli for manual simulation

extern crate crossbeam_channel;
extern crate base64;

use lib3h_ghost_actor::{GhostActor, GhostContextEndpoint};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use lib3h_protocol::Address;
use lib3h_sodium::{hash, secbuf::SecBuf};
use lib3h_crypto_api::{CryptoError};
use lib3h::engine::ghost_engine::{GhostEngine, EngineRequestToChild, EngineRequestToParent};
use lib3h_protocol::data_types::SpaceData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageData {
    pub from_address: String,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEvent {
    Message(MessageData),
    Join(Address),
    Part(Address),
}

pub type HandleEvent = Box<dyn FnMut(ChatEvent) + Send>;

pub struct HalfBusyChat {
    thread: Option<std::thread::JoinHandle<()>>,
    thread_continue: Arc<AtomicBool>,
    out_send: crossbeam_channel::Sender<ChatEvent>,
}

impl Drop for HalfBusyChat {
    fn drop(&mut self) {
        self.thread_continue.store(false, Ordering::Relaxed);
        std::mem::replace(&mut self.thread, None)
            .expect("cannot drop, thread already None")
            .join()
            .expect("thead join failed");
    }
}

impl HalfBusyChat {
    pub fn new(mut handler: HandleEvent) -> Self {
        let thread_continue = Arc::new(AtomicBool::new(true));

        let (out_send, out_recv) = crossbeam_channel::unbounded();

        let thread_continue_inner = thread_continue.clone();
        Self {
            thread: Some(std::thread::spawn(move || {
                // this thread owns the ghost engine instance
                // and is responsible for calling process
                // and handling messages
                let mut lib3h_engine = GhostEngine::new();
                let mut parent_endpoint: GhostContextEndpoint<String, _, _, _, _, _> = lib3h_engine
                    .take_parent_endpoint()
                    .unwrap()
                    .as_context_endpoint("parent");

                while thread_continue_inner.load(Ordering::Relaxed) {
                    // call process to make stuff happen
                    parent_endpoint.process(&mut ()).unwrap();
                    lib3h_engine.process().unwrap();

                    // N3HProtocol => ChatEvent
                    parent_endpoint.drain_messages().iter_mut().for_each(|message| {
                        let chat_event = match message.take_message().unwrap() {
                            EngineRequestToParent::HandleSendDirectMessage(_) => {
                                ChatEvent::Message(MessageData{from_address: "".to_string(), payload: "message from engine".to_string()})
                            }
                        };
                        // also call the handler
                        handler(chat_event);
                    });

                    // ChatAction => N3hProtocol
                    if let Ok(msg) = out_recv.try_recv() {
                        match msg {
                            ChatEvent::Join(ref addr) => {
                                let space_data = SpaceData {
                                    agent_id: Address::new(),
                                    request_id: "".to_string(),
                                    space_address: addr.to_owned(),
                                };
                                parent_endpoint.publish(EngineRequestToChild::JoinSpace(space_data))
                            },
                            ChatEvent::Part(_) => {
                            },
                            ChatEvent::Message(_) => {

                            }
                        };
                        // also call the handler
                        handler(msg);
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
        let mut chat = HalfBusyChat::new(Box::new(move |event| {
            s.send(event).expect("send fail");
        }));

        let msg = ChatEvent::Message(MessageData {
            from_address: "addr".to_string(),
            payload: "yo".to_string(),
        });

        chat.send(msg.clone());

        assert_eq!(msg, r.recv().expect("receive fail"));
    }

    #[test]
    fn can_convert_strings_to_channel_address() {
        let addr = channel_address_from_str("test channel id").expect("failed to hash string");
        assert_eq!(addr, Address::from("mgb/+MzdPWAYRs4ERGlj3WCg53AddXsg1HjXH7pk7pE=".to_string()))
    }
}

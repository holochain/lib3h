
mod handle_chat_event;
mod handle_lib3h_event;

use handle_chat_event::handle_chat_event;
use handle_lib3h_event::handle_and_convert_lib3h_event;
use crate::simchat::{ChatEvent, SimChat};

use lib3h::error::Lib3hError;
use lib3h_crypto_api::CryptoError;
use lib3h_ghost_actor::{
    GhostActor, GhostCanTrack, GhostContextEndpoint,
};
use lib3h_protocol::{
    data_types::{ConnectData, SpaceData},
    protocol::{ClientToLib3h, ClientToLib3hResponse, Lib3hToClient, Lib3hToClientResponse},
    Address,
};
use lib3h_sodium::{hash, secbuf::SecBuf};
use lib3h_tracing::TestTrace;

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

type EngineBuilder<T> = fn() -> T;
pub type CAS = HashMap<Address, String>; // space_address -> anchor_addres -> message_address

pub type HandleEvent = Box<dyn FnMut(&ChatEvent) + Send>;

pub struct Lib3hSimChat {
    thread: Option<std::thread::JoinHandle<()>>,
    thread_continue: Arc<AtomicBool>,
    out_send: crossbeam_channel::Sender<ChatEvent>,
}

pub struct Lib3hSimChatState {
    current_space: Option<SpaceData>,
    spaces: HashMap<Address, SpaceData>,
    cas: CAS,
    author_list: HashMap<Address, Vec<Address>>, // Aspect addresses per entry,
    gossip_list: HashMap<Address, Vec<Address>>, // same
}

impl Lib3hSimChatState {
    pub fn new() -> Self {
        Self {
            current_space: None,
            spaces: HashMap::new(),
            cas: HashMap::new(),
            author_list: HashMap::new(),
            gossip_list: HashMap::new(),
        }
    }
}

impl Drop for Lib3hSimChat {
    fn drop(&mut self) {
        self.thread_continue.store(false, Ordering::Relaxed);
        std::mem::replace(&mut self.thread, None)
            .expect("cannot drop, thread already None")
            .join()
            .expect("thead join failed");
    }
}

impl Lib3hSimChat {
    pub fn new<T>(engine_builder: EngineBuilder<T>, mut handler: HandleEvent, peer_uri: Url) -> Self
    where
        T: GhostActor<
                Lib3hToClient,
                Lib3hToClientResponse,
                ClientToLib3h,
                ClientToLib3hResponse,
                Lib3hError,
            > + 'static,
    {
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
                let mut engine = engine_builder();

                let mut parent_endpoint: GhostContextEndpoint<(), TestTrace, _, _, _, _, _> =
                    engine
                        .take_parent_endpoint()
                        .unwrap()
                        .as_context_endpoint_builder()
                        .request_id_prefix("parent")
                        .build();

                // also keep track of things like the spaces and current space in this scope
                let mut state = Lib3hSimChatState::new();

                // call connect to start the networking process
                // (should probably wait for confirmatio before continuing)
                Self::connect(&mut parent_endpoint, peer_uri);

                while thread_continue_inner.load(Ordering::Relaxed) {
                    // call process to make stuff happen
                    parent_endpoint.process(&mut ()).unwrap();
                    engine.process().unwrap();

                    // grab any new events from lib3h
                    let engine_chat_events = parent_endpoint
                        .drain_messages();

                    // gather all the ChatEvents
                    // Receive directly from the crossbeam channel
                    // and convert relevent N3H protocol messages to chat events and handle
                    let direct_chat_events = out_recv.try_iter();
                    let _engine_chat_events = engine_chat_events
                        .into_iter()
                        // process lib3h messages and convert to a chat event if required
                        .for_each(|engine_message| {
                            if let Some(chat_event) = handle_and_convert_lib3h_event(engine_message, &mut state) {
                            	handler(&chat_event);
                        		handle_chat_event(chat_event, &mut state, &mut parent_endpoint, internal_sender.clone());
                            }
                        });

                    // process all the chat events by calling the handler for all events
                    // and dispatching new n3h actions where required
                    for chat_event in direct_chat_events {

                        // every chat event call the handler that was passed
                        handler(&chat_event);

                        // also do internal logic for certain events e.g. converting them to lib3h events
                        // and also handling the responses to mutate local state
                        handle_chat_event(chat_event, &mut state, &mut parent_endpoint, internal_sender.clone());
                    }

                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            })),
            thread_continue,
            out_send,
        }
    }

    fn connect(
        endpoint: &mut GhostContextEndpoint<
            (),
            TestTrace,
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hToClient,
            Lib3hToClientResponse,
            Lib3hError,
        >,
        peer_uri: Url,
    ) {
        let connect_message = ClientToLib3h::Connect(ConnectData {
            network_id: String::from(""), // connect to any
            peer_uri,
            request_id: String::from("connect-request"),
        });
        endpoint
            .request(
                TestTrace::new(),
                connect_message,
                Box::new(|_, callback_data| {
                    println!("chat received response from engine: {:?}", callback_data);
                    Ok(())
                }),
            )
            .unwrap();
    }
}

impl SimChat for Lib3hSimChat {
    fn send(&mut self, event: ChatEvent) {
        self.out_send.send(event).expect("send fail");
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

pub fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

#[cfg(test)]
mod tests {
	pub mod mock_engine;

    use crate::simchat::SimChatMessage;
	use super::*;

    fn new_sim_chat_mock_engine(callback: HandleEvent) -> Lib3hSimChat {
        Lib3hSimChat::new(
            mock_engine::MockEngine::new,
            callback,
            Url::parse("http://test.boostrap").unwrap(),
        )
    }

    /*----------  example messages  ----------*/

    fn chat_event() -> ChatEvent {
        ChatEvent::SendDirectMessage {
            to_agent: "addr".to_string(),
            payload: "yo".to_string(),
        }
    }

    fn receive_sys_message(payload: String) -> ChatEvent {
        ChatEvent::ReceiveDirectMessage(SimChatMessage{
            from_agent: String::from("sys"),
            payload,
            timestamp: current_timestamp(),
        })
    }

    fn join_event() -> ChatEvent {
        ChatEvent::Join {
            agent_id: "test_agent".to_string(),
            channel_id: "test_channel".to_string(),
        }
    }

    fn join_success_event() -> ChatEvent {
        let space_address = channel_address_from_string(&"test_channel".to_string())
            .expect("failed to hash string");

        ChatEvent::JoinSuccess {
            channel_id: "test_channel".to_string(),
            space_data: SpaceData {
                agent_id: Address::from("test_agent"),
                request_id: "".to_string(),
                space_address,
            },
        }
    }

    fn part_event() -> ChatEvent {
        ChatEvent::Part("test_channel".to_string())
    }

    fn part_success_event() -> ChatEvent {
        ChatEvent::PartSuccess("test_channel".to_string())
    }

    #[test]
    fn it_should_echo() {
        let (s, r) = crossbeam_channel::unbounded();
        let mut chat = new_sim_chat_mock_engine(Box::new(move |event| {
            s.send(event.to_owned()).expect("send fail");
        }));
        chat.send(chat_event());
        assert_eq!(chat_event(), r.recv().expect("receive fail"));
    }

    /*----------  join/part ----------*/

    #[test]
    fn calling_join_space_triggers_success_message() {
        let (s, r) = crossbeam_channel::unbounded();
        let mut chat = new_sim_chat_mock_engine(Box::new(move |event| {
            s.send(event.to_owned()).expect("send fail");
        }));

        chat.send(join_event());

        let chat_messages = r.iter().take(2).collect::<Vec<_>>();
        assert_eq!(chat_messages, vec![join_event(), join_success_event(),],);
    }

    #[test]
    fn calling_part_before_join_fails() {
        let (s, r) = crossbeam_channel::unbounded();
        let mut chat = new_sim_chat_mock_engine(Box::new(move |event| {
            s.send(event.to_owned()).expect("send fail");
        }));

        chat.send(part_event());

        let chat_messages = r.iter().take(2).collect::<Vec<_>>();
        assert_eq!(
            chat_messages,
            vec![
                part_event(),
                receive_sys_message("No channel to leave".to_string()),
            ],
        );
    }

    #[test]
    fn calling_join_then_part_succeeds() {
        let (s, r) = crossbeam_channel::unbounded();
        let mut chat = new_sim_chat_mock_engine(Box::new(move |event| {
            s.send(event.to_owned()).expect("send fail");
        }));

        chat.send(join_event());
        std::thread::sleep(std::time::Duration::from_millis(100)); // find a better way
        chat.send(part_event());

        let chat_messages = r.iter().take(5).collect::<Vec<_>>();
        assert_eq!(
            chat_messages,
            vec![
                join_event(),
                join_success_event(),
                receive_sys_message("Joined channel: test_channel".to_string()),
                part_event(),
                part_success_event(),
            ],
        );
    }

    /*----------  direct message  ----------*/

    #[test]
    fn sending_direct_message_before_join_fails() {
        let (s, r) = crossbeam_channel::unbounded();
        let mut chat = new_sim_chat_mock_engine(Box::new(move |event| {
            s.send(event.to_owned()).expect("send fail");
        }));

        chat.send(chat_event());

        let chat_messages = r.iter().take(2).collect::<Vec<_>>();
        assert_eq!(
            chat_messages,
            vec![
                chat_event(),
                receive_sys_message("Must join a channel before sending a message".to_string()),
            ],
        );
    }

    #[test]
    fn can_join_and_send_direct_message() {
        let (s, r) = crossbeam_channel::unbounded();
        let mut chat = new_sim_chat_mock_engine(Box::new(move |event| {
            s.send(event.to_owned()).expect("send fail");
        }));

        chat.send(join_event());
        std::thread::sleep(std::time::Duration::from_millis(100)); // find a better way
        chat.send(chat_event());

        let chat_messages = r.iter().take(4).collect::<Vec<_>>();
        assert_eq!(
            chat_messages,
            vec![
                join_event(),
                join_success_event(),
                receive_sys_message("Joined channel: test_channel".to_string()),
                chat_event(),
            ],
        );
    }

    #[test]
    fn can_convert_strings_to_channel_address() {
        let addr = channel_address_from_string(&String::from("test channel id"))
            .expect("failed to hash string");
        assert_eq!(
            addr,
            Address::from("mgb/+MzdPWAYRs4ERGlj3WCg53AddXsg1HjXH7pk7pE=".to_string())
        )
    }
}


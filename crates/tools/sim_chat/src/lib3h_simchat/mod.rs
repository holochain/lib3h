mod handle_chat_event;
mod handle_lib3h_event;
mod store;

use crate::simchat::{ChatEvent, SimChat, SimChatMessage};
use handle_chat_event::handle_chat_event;
use handle_lib3h_event::handle_and_convert_lib3h_event;
use lib3h::{engine::CanAdvertise, error::Lib3hError};
use lib3h_crypto_api::CryptoError;
use lib3h_protocol::{
    data_types::SpaceData,
    protocol::{ClientToLib3h, ClientToLib3hResponse, Lib3hToClient, Lib3hToClientResponse},
    Address,
};
use lib3h_sodium::{hash, secbuf::SecBuf};
// use lib3h_tracing::test_span;
use lib3h_zombie_actor::{GhostActor, GhostCanTrack, GhostContextEndpoint};
use store::{Store, StoreEntryList};

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use url::Url;

type EngineBuilder<T> = fn() -> T;

pub type HandleEvent = Box<dyn FnMut(&ChatEvent) + Send>;

pub struct Lib3hSimChat {
    thread: Option<std::thread::JoinHandle<()>>,
    thread_continue: Arc<AtomicBool>,
    out_send: crossbeam_channel::Sender<ChatEvent>,
}

pub struct Lib3hSimChatState {
    /// Is the current lib3h engine reporting connected status
    connected: bool,

    /// Stores the space that messages will be posted to by default
    /// This should also be the space on the prompt if not None
    current_space: Option<SpaceData>,

    /// All spaces that are joined. It is important to keep track of these
    /// as attempting to rejoin a space causes an error
    spaces: HashMap<Address, SpaceData>,

    /// Replacement for Holochain CAS. This stores messages on time anchors
    /// It is a multi-level hash table where messages are indexed by
    /// space-address -> time-anchor-address -> message-address => MessageData
    store: Store,

    /// These are the messages that a ReceiveChannelMessage event has already
    /// been triggered for. This ensures that only one ReceiveChannelEvent is
    /// emitted per message per session
    displayed_channel_messages: Vec<Address>,

    /// Maintains a list of the entries and aspects authored by this agent
    /// space-address -> entry-address => aspect-address
    author_list: StoreEntryList,

    /// Similarly maintains a list of entries and aspects received via gossip
    /// Same indexing scheme
    gossip_list: StoreEntryList,
}

impl Lib3hSimChatState {
    pub fn new() -> Self {
        Self {
            connected: false,
            current_space: None,
            spaces: HashMap::new(),
            store: Store::new(),
            displayed_channel_messages: Vec::new(),
            author_list: StoreEntryList::new(),
            gossip_list: StoreEntryList::new(),
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
    pub fn new<T>(
        engine_builder: EngineBuilder<T>,
        mut handler: HandleEvent,
        bootstrap_uris: Vec<Url>,
    ) -> Self
    where
        T: GhostActor<
                Lib3hToClient,
                Lib3hToClientResponse,
                ClientToLib3h,
                ClientToLib3hResponse,
                Lib3hError,
            > + CanAdvertise
            + 'static,
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

                println!("Engine initialized with peer URI: {}", engine.advertise());

                let mut parent_endpoint: GhostContextEndpoint<(), _, _, _, _, _> = engine
                    .take_parent_endpoint()
                    .expect("Could not get parent endpoint")
                    .as_context_endpoint_builder()
                    .request_id_prefix("parent")
                    .build();

                // also keep track of things like the spaces and current space in this scope
                let mut state = Lib3hSimChatState::new();

                // If any bootstrap nodes were provided try and use them
                for peer_uri in bootstrap_uris {
                    Self::bootstrap(peer_uri, internal_sender.clone());
                }

                while thread_continue_inner.load(Ordering::Relaxed) {
                    // call process to make stuff happen
                    parent_endpoint.process(&mut ()).ok();
                    engine.process().ok();

                    // grab any new events from lib3h
                    let engine_chat_events = parent_endpoint.drain_messages();

                    // gather all the ChatEvents
                    // Receive directly from the crossbeam channel
                    // and convert relevent N3H protocol messages to chat events and handle
                    let direct_chat_events = out_recv.try_iter();
                    let _engine_chat_events = engine_chat_events
                        .into_iter()
                        // process lib3h messages and convert to a chat event if required
                        .for_each(|mut engine_message| {
                            if let Some(lib3h_message) = engine_message.take_message() {
                                let (maybe_chat_event, maybe_response) =
                                    handle_and_convert_lib3h_event(lib3h_message, &mut state);

                                if let Some(response) = maybe_response {
                                    engine_message
                                        .respond(response)
                                        .expect("Could not send response!");
                                }
                                if let Some(chat_event) = maybe_chat_event {
                                    // call the external handler for every resulting event
                                    handler(&chat_event);
                                    // call the internal handler also
                                    handle_chat_event(
                                        chat_event,
                                        &mut state,
                                        internal_sender.clone(),
                                    )
                                    .map(|lib3h_request| {
                                        lib3h_request.execute_request(&mut parent_endpoint)
                                    });
                                }
                            }
                        });

                    // process all the chat events by calling the handler for all events
                    // and dispatching new n3h actions where required
                    for chat_event in direct_chat_events {
                        // every chat event call the handler that was passed
                        handler(&chat_event);

                        // also do internal logic for certain events e.g. converting them to lib3h events
                        // and also handling the responses to mutate local state
                        handle_chat_event(chat_event, &mut state, internal_sender.clone()).map(
                            |lib3h_request| lib3h_request.execute_request(&mut parent_endpoint),
                        );
                    }

                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            })),
            thread_continue,
            out_send,
        }
    }

    /// Uses lib3h to try and bootstrap via the given URI
    /// This will trigger a sys message on dispatch and success
    fn bootstrap(bootstrap_uri: Url, chat_message_sender: crossbeam_channel::Sender<ChatEvent>) {
        send_sys_message(
            chat_message_sender.clone(),
            &format!("Attempting to boostrap via {}", bootstrap_uri),
        );
        chat_message_sender
            .send(ChatEvent::Bootstrap(bootstrap_uri))
            .ok();
    }
}

impl SimChat for Lib3hSimChat {
    fn send(&mut self, event: ChatEvent) {
        self.out_send.send(event).expect("send fail");
    }
}

pub fn send_sys_message(sender: crossbeam_channel::Sender<ChatEvent>, msg: &String) {
    sender
        .send(ChatEvent::ReceiveDirectMessage(SimChatMessage {
            from_agent: String::from("sys"),
            payload: String::from(msg),
            timestamp: current_timestamp(),
        }))
        .expect("send fail");
}

pub fn channel_address_from_string(channel_id: &String) -> Result<Address, CryptoError> {
    let mut input = SecBuf::with_insecure_from_string(channel_id.to_string());
    let mut output = SecBuf::with_insecure(hash::BYTES256);
    hash::sha256(&mut input, &mut output).unwrap();
    let buf = output.read_lock();
    let signature_str = base64::encode(&**buf);
    Ok(Address::from(signature_str))
}

#[cfg(not(test))]
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)] // make sure time doesn't mess up the tests
pub fn current_timestamp() -> u64 {
    0
}

pub fn current_timeanchor() -> Address {
    Address::from("0")
}

#[cfg(test)]
mod tests {
    pub mod mock_engine;

    use super::*;
    use crate::simchat::SimChatMessage;

    fn new_sim_chat_mock_engine(callback: HandleEvent) -> Lib3hSimChat {
        Lib3hSimChat::new(mock_engine::MockEngine::new, callback, vec![])
    }

    /*----------  example messages  ----------*/

    fn chat_event() -> ChatEvent {
        ChatEvent::SendDirectMessage {
            to_agent: "addr".to_string(),
            payload: "yo".to_string(),
        }
    }

    fn receive_sys_message(payload: String) -> ChatEvent {
        ChatEvent::ReceiveDirectMessage(SimChatMessage {
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
        ChatEvent::Part {
            channel_id: "test_channel".to_string(),
        }
    }

    fn part_success_event() -> ChatEvent {
        ChatEvent::PartSuccess {
            channel_id: "test_channel".to_string(),
        }
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
                receive_sys_message("You joined the channel: test_channel".to_string()),
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
                receive_sys_message("You joined the channel: test_channel".to_string()),
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

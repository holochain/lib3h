use super::current_timestamp;
use crate::{
    lib3h_simchat::Lib3hSimChatState,
    simchat::{ChatEvent, SimChatMessage},
};
use lib3h::error::Lib3hError;
use lib3h_ghost_actor::GhostMessage;
use lib3h_protocol::{
    data_types::*,
    protocol::{ClientToLib3h, Lib3hToClient, Lib3hToClientResponse},
};

pub fn handle_and_convert_lib3h_event(
    mut engine_message: GhostMessage<
        Lib3hToClient,
        ClientToLib3h,
        Lib3hToClientResponse,
        Lib3hError,
    >,
    state: &mut Lib3hSimChatState,
) -> Option<ChatEvent> {
    match engine_message.take_message() {
        Some(Lib3hToClient::HandleQueryEntry(QueryEntryData {
            // currently only one query which returns all messages for a time anchor
            space_address,
            entry_address,
            request_id,
            requester_agent_id,
            query: _,
        })) => {
            // respond with your ID in this space
            let responder_agent_id = state.spaces.get(&space_address).unwrap().agent_id.clone();
            if let Some(entry) = state.store.get_all_messages(&space_address, &entry_address) {
                // load and return data from the CAS
                engine_message
                    .respond(Ok(Lib3hToClientResponse::HandleQueryEntryResult(
                        QueryEntryResultData {
                            request_id,
                            entry_address,
                            requester_agent_id: requester_agent_id.clone(),
                            space_address: space_address.clone(),
                            responder_agent_id,
                            query_result: entry.to_opaque(),
                        },
                    )))
                    .ok();
            } else {
                // is this the correct way to respond to a missing entry?
                engine_message
                    .respond(Err(Lib3hError::new_other("Entry not found")))
                    .ok();
            }
            None
        }
        Some(Lib3hToClient::HandleStoreEntryAspect(StoreEntryAspectData {
            entry_address,
            entry_aspect,
            space_address,
            ..
        })) => {
            // store a new message on a possibly existing time anchor entry
            state.store.insert(
                &space_address,
                &entry_address,
                &entry_aspect.aspect_address,
                SimChatMessage::from_opaque(entry_aspect.aspect),
            );
            engine_message
                .respond(Ok(Lib3hToClientResponse::HandleStoreEntryAspectResult))
                .ok();
            None
        }
        // Some(Lib3hToClient::HandleGetAuthoringEntryList(GetListData {
        //     request_id,
        //     space_address,
        //     provider_agent_id,
        // })) => {
        //     engine_message
        //         .respond(Ok(
        //             Lib3hToClientResponse::HandleGetAuthoringEntryListResult(EntryListData {
        //                 request_id,
        //                 space_address,
        //                 provider_agent_id,
        //                 address_map: state.author_list.clone(),
        //             }),
        //         ))
        //         .ok();
        //     None
        // }
        // Some(Lib3hToClient::HandleGetGossipingEntryList(GetListData {
        //     request_id,
        //     space_address,
        //     provider_agent_id,
        // })) => {
        //     engine_message
        //         .respond(Ok(
        //             Lib3hToClientResponse::HandleGetAuthoringEntryListResult(EntryListData {
        //                 request_id,
        //                 space_address,
        //                 provider_agent_id,
        //                 address_map: state.gossip_list.clone(),
        //             }),
        //         ))
        //         .ok();
        //     None
        // }
        Some(Lib3hToClient::HandleSendDirectMessage(message_data)) => {
            Some(ChatEvent::ReceiveDirectMessage(SimChatMessage {
                from_agent: message_data.from_agent_id.to_string(),
                payload: "message from engine".to_string(),
                timestamp: current_timestamp(),
            }))
        }
        Some(Lib3hToClient::Disconnected(_)) => Some(ChatEvent::Disconnected),
        Some(_) => None, // event we don't care about
        None => None,    // there was nothing in the message
    }
}

#[cfg(test)]
pub mod test {
    use crate::simchat::{SimChatMessage, MessageList};
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::collections::HashSet;
    use std::hash::{Hash, Hasher};
    use std::iter::FromIterator;
    use lib3h_protocol::{
        Address,
    };    

    fn send_message_event(message: &SimChatMessage) -> Lib3hToClient {
        let mut hasher = DefaultHasher::new();
        message.hash(&mut hasher);
        Lib3hToClient::HandleStoreEntryAspect(
            StoreEntryAspectData {
                space_address: Address::from("some_space"),
                entry_address: Address::from("some_entry"),
                request_id: String::from("0"),
                provider_agent_id: Address::from("some_agent"),
                entry_aspect: EntryAspectData {
                    aspect: message.to_opaque(),
                    aspect_address: Address::from(hasher.finish().to_string()),
                    type_hint: String::from(""),
                    publish_ts: 0,
                }
            }
        )
    }

    #[test]
    fn handle_store_entry_mutates_state() {
        let mut state = Lib3hSimChatState::new();
        let message = SimChatMessage {
            from_agent: String::from("some_agent"),
            payload: String::from("hi"),
            timestamp: 0,
        };
        let (s,_r) = crossbeam_channel::unbounded();
        let ghost_message = GhostMessage::new(None, send_message_event(&message), s);
        let response = handle_and_convert_lib3h_event(ghost_message, &mut state);

        assert_eq!(response, None); // this does not produce a chat event
        assert_eq!(
            state.store.get_all_messages(&Address::from("some_space"), &Address::from("some_entry")).expect("no messages"),
            MessageList(vec![message])
        )
    }

    #[test]
    fn messages_stack_in_store() {
        let mut state = Lib3hSimChatState::new();
        let messages = vec![
            SimChatMessage {
                from_agent: String::from("some_agent"),
                payload: String::from("hi"),
                timestamp: 0,
            },
            SimChatMessage {
                from_agent: String::from("some_agent"),
                payload: String::from("hi2"),
                timestamp: 1,
            }
        ];

        let (s,_r) = crossbeam_channel::unbounded();
        for message in &messages {
            let ghost_message = GhostMessage::new(None, send_message_event(&message), s.clone());
            handle_and_convert_lib3h_event(ghost_message, &mut state);
        }

        let stored_messages: Vec<SimChatMessage> = state.store.get_all_messages(&Address::from("some_space"), &Address::from("some_entry")).expect("no messages").0;

        assert_eq!(
            HashSet::<SimChatMessage>::from_iter(stored_messages.into_iter()), 
            HashSet::from_iter(messages.into_iter())
        )
    }

    #[test]
    fn messages_dedup_in_store() {
        let mut state = Lib3hSimChatState::new();
        let messages = vec![
            SimChatMessage {
                from_agent: String::from("some_agent"),
                payload: String::from("hi"),
                timestamp: 0,
            },
            SimChatMessage {
                from_agent: String::from("some_agent"),
                payload: String::from("hi"),
                timestamp: 0,
            }
        ];

        let (s,_r) = crossbeam_channel::unbounded();
        for message in &messages {
            let ghost_message = GhostMessage::new(None, send_message_event(&message), s.clone());
            handle_and_convert_lib3h_event(ghost_message, &mut state);
        }

        let stored_messages: Vec<SimChatMessage> = state.store.get_all_messages(&Address::from("some_space"), &Address::from("some_entry")).expect("no messages").0;

        assert_eq!(
            stored_messages, 
            vec![messages[0].clone()]
        )
    }
}

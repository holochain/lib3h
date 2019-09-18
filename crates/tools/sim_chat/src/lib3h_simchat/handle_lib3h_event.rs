use super::current_timestamp;
use crate::{
    lib3h_simchat::Lib3hSimChatState,
    simchat::{ChatEvent, OpaqueConvertable, SimChatMessage},
};
use lib3h::error::{Lib3hError, Lib3hResult};
use lib3h_protocol::{
    data_types::*,
    protocol::{Lib3hToClient, Lib3hToClientResponse},
};

///
pub fn handle_and_convert_lib3h_event(
    lib3h_message: Lib3hToClient,
    state: &mut Lib3hSimChatState,
) -> (
    Option<ChatEvent>,
    Option<Lib3hResult<Lib3hToClientResponse>>,
) {
    match lib3h_message {
        Lib3hToClient::HandleQueryEntry(QueryEntryData {
            // currently only one query which returns all messages for a time anchor
            space_address,
            entry_address,
            request_id,
            requester_agent_id,
            query: _,
        }) => {
            // respond with your ID in this space
            let responder_agent_id = state
                .spaces
                .get(&space_address)
                .expect("This agent does not track this space")
                .agent_id
                .clone();
            if let Some(entry) = state.store.get_all_messages(&space_address, &entry_address) {
                (
                    None,
                    Some(Ok(Lib3hToClientResponse::HandleQueryEntryResult(
                        QueryEntryResultData {
                            request_id,
                            entry_address,
                            requester_agent_id: requester_agent_id.clone(),
                            space_address: space_address.clone(),
                            responder_agent_id,
                            query_result: entry.to_opaque(),
                        },
                    ))),
                )
            } else {
                // is this the correct way to respond to a missing entry?
                (None, Some(Err(Lib3hError::new_other("Entry not found"))))
            }
        }

        Lib3hToClient::HandleStoreEntryAspect(StoreEntryAspectData {
            entry_address,
            entry_aspect,
            space_address,
            ..
        }) => {
            // store a new message on a possibly existing time anchor entry
            state.store.insert(
                &space_address,
                &entry_address,
                &entry_aspect.aspect_address,
                SimChatMessage::from_opaque(entry_aspect.aspect),
            );

            // Update the gossip list
            state
                .gossip_list
                .insert(&space_address, &entry_address, &entry_aspect.aspect_address);

            (
                None,
                Some(Ok(Lib3hToClientResponse::HandleStoreEntryAspectResult)),
            )
        }

        Lib3hToClient::HandleSendDirectMessage(message_data) => (
            Some(ChatEvent::ReceiveDirectMessage(SimChatMessage {
                from_agent: message_data.from_agent_id.to_string(),
                payload: "message from engine".to_string(),
                timestamp: current_timestamp(),
            })),
            None,
        ),

        Lib3hToClient::Connected(_) => {
            state.connected = true;
            (Some(ChatEvent::Connected), None)
        }

        Lib3hToClient::Disconnected(_) => {
            state.connected = false;
            (Some(ChatEvent::Disconnected), None)
        }

        Lib3hToClient::HandleGetAuthoringEntryList(GetListData {
            space_address,
            request_id,
            provider_agent_id,
        }) => (
            None,
            Some(Ok(
                Lib3hToClientResponse::HandleGetAuthoringEntryListResult(EntryListData {
                    request_id,
                    space_address: space_address.clone(),
                    provider_agent_id,
                    address_map: state
                        .author_list
                        .get(&space_address)
                        .map(|s| s.to_owned())
                        .unwrap_or_default(),
                }),
            )),
        ),

        Lib3hToClient::HandleGetGossipingEntryList(GetListData {
            space_address,
            request_id,
            provider_agent_id,
        }) => (
            None,
            Some(Ok(
                Lib3hToClientResponse::HandleGetGossipingEntryListResult(EntryListData {
                    request_id,
                    space_address: space_address.clone(),
                    provider_agent_id,
                    address_map: state
                        .gossip_list
                        .get(&space_address)
                        .map(|s| s.to_owned())
                        .unwrap_or_default(),
                }),
            )),
        ),

        _ => (None, None), // event we don't care about
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::simchat::{MessageList, SimChatMessage};
    use std::{collections::HashSet, iter::FromIterator};

    use lib3h_protocol::Address;

    fn store_message_event(message: &SimChatMessage) -> Lib3hToClient {
        Lib3hToClient::HandleStoreEntryAspect(StoreEntryAspectData {
            space_address: Address::from("some_space"),
            entry_address: Address::from("some_entry"),
            request_id: String::from("0"),
            provider_agent_id: Address::from("some_agent"),
            entry_aspect: EntryAspectData {
                aspect: message.to_opaque(),
                aspect_address: message.address(),
                type_hint: String::from(""),
                publish_ts: 0,
            },
        })
    }

    fn retrieve_messages_event(space_address: &Address, base_address: &Address) -> Lib3hToClient {
        Lib3hToClient::HandleQueryEntry(QueryEntryData {
            space_address: space_address.clone(),
            entry_address: base_address.clone(),
            request_id: String::from("0"),
            query: Opaque::new(),
            requester_agent_id: Address::from("some_agent"),
        })
    }

    /*----------  Storing messages  ----------*/

    #[test]
    fn handle_store_entry_mutates_state() {
        let mut state = Lib3hSimChatState::new();
        let message = SimChatMessage {
            from_agent: String::from("some_agent"),
            payload: String::from("hi"),
            timestamp: 0,
        };
        let response = handle_and_convert_lib3h_event(store_message_event(&message), &mut state);

        assert_eq!(response.0, None); // this does not produce a chat event
        assert_eq!(
            // state has been updated
            state
                .store
                .get_all_messages(&Address::from("some_space"), &Address::from("some_entry"))
                .expect("no messages"),
            MessageList(vec![message.clone()])
        );
        assert_eq!(
            // also updates gossip list
            state
                .gossip_list
                .get(&Address::from("some_space"))
                .expect("Could not get space hashmap")
                .get(&Address::from("some_entry")),
            Some(&vec![message.address()]),
        );
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
            },
        ];

        for message in &messages {
            handle_and_convert_lib3h_event(store_message_event(&message), &mut state);
        }

        let stored_messages: Vec<SimChatMessage> = state
            .store
            .get_all_messages(&Address::from("some_space"), &Address::from("some_entry"))
            .expect("no messages")
            .0;

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
            },
        ];

        for message in &messages {
            handle_and_convert_lib3h_event(store_message_event(&message), &mut state);
        }

        let stored_messages: Vec<SimChatMessage> = state
            .store
            .get_all_messages(&Address::from("some_space"), &Address::from("some_entry"))
            .expect("no messages")
            .0;

        assert_eq!(stored_messages, vec![messages[0].clone()])
    }

    /*----------  retrieving messages  ----------*/

    #[test]
    fn can_query_and_get_message() {
        let mut state = Lib3hSimChatState::new();
        let message = SimChatMessage {
            from_agent: String::from("some_agent"),
            payload: String::from("hi"),
            timestamp: 0,
        };
        // must actually be tracking the space the entry is requested from
        state.spaces.insert(
            Address::from("some_space"),
            SpaceData {
                agent_id: Address::from("holder_agent"),
                request_id: String::from("0"),
                space_address: Address::from("some_space"),
            },
        );
        state.store.insert(
            &Address::from("some_space"),
            &Address::from("some_entry"),
            &Address::from("message_address"),
            message.clone(),
        );

        let result = handle_and_convert_lib3h_event(
            retrieve_messages_event(&Address::from("some_space"), &Address::from("some_entry")),
            &mut state,
        );

        assert_eq!(result.0, None); // no chat events for this event

        let query_result = match result
            .1
            .expect("no response from handler")
            .expect("handler returned error")
        {
            Lib3hToClientResponse::HandleQueryEntryResult(QueryEntryResultData {
                query_result,
                ..
            }) => query_result,
            _ => panic!("wrong response type"),
        };

        assert_eq!(
            MessageList::from_opaque(query_result),
            MessageList(vec![message])
        );
    }

}

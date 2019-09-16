use lib3h_tracing::test_span;

use crate::lib3h_simchat::current_timeanchor;
use lib3h_protocol::protocol::{Lib3hToClient, Lib3hToClientResponse};

use super::{channel_address_from_string, current_timestamp, Lib3hSimChatState};
use crate::simchat::{ChatEvent, MessageList, OpaqueConvertable, SimChatMessage};
use lib3h::error::Lib3hError;
use lib3h_protocol::{
    data_types::*,
    protocol::{ClientToLib3h, ClientToLib3hResponse},
    Address,
};

use lib3h_zombie_actor::{
    GhostCallback, GhostCallbackData::Response, GhostCanTrack, GhostContextEndpoint, GhostResult,
};

pub struct Lib3hEventAndCallback {
    event: ClientToLib3h,
    callback: GhostCallback<(), ClientToLib3hResponse, Lib3hError>,
}

impl Lib3hEventAndCallback {
    pub fn new(
        event: ClientToLib3h,
        callback: GhostCallback<(), ClientToLib3hResponse, Lib3hError>,
    ) -> Self {
        Self { event, callback }
    }

    pub fn execute_request(
        self,
        endpoint: &mut GhostContextEndpoint<
            (),
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hToClient,
            Lib3hToClientResponse,
            Lib3hError,
        >,
    ) -> GhostResult<()> {
        endpoint.request(test_span(""), self.event, self.callback)
    }
}

pub fn handle_chat_event(
    chat_event: ChatEvent,
    state: &mut Lib3hSimChatState,
    chat_event_sender: crossbeam_channel::Sender<ChatEvent>,
) -> Option<Lib3hEventAndCallback> {
    match chat_event {
        ChatEvent::Join {
            channel_id,
            agent_id,
        } => {
            let space_address = channel_address_from_string(&channel_id).unwrap();
            let space_data = SpaceData {
                agent_id: Address::from(agent_id),
                request_id: "".to_string(),
                space_address,
            };
            Some(Lib3hEventAndCallback::new(
                ClientToLib3h::JoinSpace(space_data.clone()),
                Box::new(move |_, callback_data| {
                    if let Response(Ok(_payload)) = callback_data {
                        chat_event_sender
                            .send(ChatEvent::JoinSuccess {
                                channel_id: channel_id.clone(),
                                space_data: space_data.clone(),
                            })
                            .unwrap();
                    }
                    Ok(())
                }),
            ))
        }

        ChatEvent::JoinSuccess {
            space_data,
            channel_id,
        } => {
            state.spaces.insert(
                channel_address_from_string(&channel_id).unwrap(),
                space_data.clone(),
            );
            state.current_space = Some(space_data);
            send_sys_message(
                chat_event_sender,
                &format!("You joined the channel: {}", channel_id),
            );
            None
        }

        ChatEvent::Part { channel_id } => {
            let space_address = channel_address_from_string(&channel_id).unwrap();
            if let Some(space_data) = state.spaces.get(&space_address) {
                Some(Lib3hEventAndCallback::new(
                    ClientToLib3h::LeaveSpace(space_data.to_owned()),
                    Box::new(move |_, callback_data| {
                        if let Response(Ok(_payload)) = callback_data {
                            chat_event_sender
                                .send(ChatEvent::PartSuccess {
                                    channel_id: channel_id.clone(),
                                })
                                .unwrap();
                        }
                        Ok(())
                    }),
                ))
            } else {
                send_sys_message(chat_event_sender, &"No channel to leave".to_string());
                None
            }
        }

        ChatEvent::PartSuccess { channel_id } => {
            let space_data = state
                .spaces
                .remove(&channel_address_from_string(&channel_id).unwrap());
            // if you just left the current space set it to None
            if state.current_space == space_data {
                state.current_space = None;
            }
            send_sys_message(
                chat_event_sender,
                &format!("You left the channel: {}", channel_id).to_string(),
            );
            None
        }

        ChatEvent::SendDirectMessage { to_agent, payload } => {
            if let Some(space_data) = state.current_space.clone() {
                let direct_message_data = DirectMessageData {
                    from_agent_id: space_data.agent_id,
                    content: payload.into(),
                    to_agent_id: Address::from(to_agent),
                    space_address: space_data.space_address,
                    request_id: String::from(""),
                };
                Some(Lib3hEventAndCallback::new(
                    ClientToLib3h::SendDirectMessage(direct_message_data),
                    Box::new(|_, _| Ok(())), // TODO: Add checking that message send was a success
                ))
            } else {
                send_sys_message(
                    chat_event_sender,
                    &"Must join a channel before sending a message".to_string(),
                );
                None
            }
        }

        ChatEvent::SendChannelMessage { payload } => {
            if let Some(space_data) = state.current_space.clone() {
                let message = SimChatMessage {
                    from_agent: space_data.agent_id.to_string(),
                    payload,
                    timestamp: current_timestamp(),
                };

                let provided_entry_data = ProvidedEntryData {
                    space_address: space_data.space_address,
                    provider_agent_id: space_data.agent_id,
                    entry: EntryData {
                        entry_address: current_timeanchor(),
                        aspect_list: vec![EntryAspectData {
                            type_hint: String::from(""),
                            aspect: message.to_opaque(),
                            aspect_address: message.address(),
                            publish_ts: current_timestamp(),
                        }],
                    },
                };

                Some(Lib3hEventAndCallback::new(
                    ClientToLib3h::PublishEntry(provided_entry_data),
                    Box::new(|_, _| Ok(())),
                ))

            // TODO: Update the author list
            } else {
                send_sys_message(
                    chat_event_sender,
                    &"Must join a channel before sending a message".to_string(),
                );
                None
            }
        }
        // TODO: Update to actually check a given time anchor
        ChatEvent::QueryChannelMessages { .. } => {
            let displayed_channel_messages = state.displayed_channel_messages.clone();
            if let Some(space_data) = state.current_space.clone() {
                // calculate which time anchors we need to be looking at
                let time_anchor_address = current_timeanchor(); // all are the same for now
                let query_entry_data = QueryEntryData {
                    query: Opaque::new(),
                    entry_address: time_anchor_address,
                    request_id: String::from(""),
                    space_address: space_data.space_address,
                    requester_agent_id: space_data.agent_id,
                };
                return Some(Lib3hEventAndCallback::new(
                    ClientToLib3h::QueryEntry(query_entry_data),
                    Box::new(move |_, callback_data| {
                        if let Response(Ok(ClientToLib3hResponse::QueryEntryResult(query_result))) =
                            callback_data
                        {
                            let messages = MessageList::from_opaque(query_result.query_result);
                            for message in messages.0 {
                                // Only emit ReceiveChannelMessage once per time seeing a message
                                if !displayed_channel_messages.contains(&message.address()) {
                                    chat_event_sender
                                        .send(ChatEvent::ReceiveChannelMessage(message))
                                        .ok();
                                }
                            }
                        }
                        Ok(())
                    }),
                ));
            } else {
                None
            }
        }

        ChatEvent::ReceiveChannelMessage(message) => {
            state.displayed_channel_messages.push(message.address());
            None
        }

        ChatEvent::Connected => {
            state.connected = true;
            None
        }

        _ => None,
    }
}

fn send_sys_message(sender: crossbeam_channel::Sender<ChatEvent>, msg: &String) {
    sender
        .send(ChatEvent::ReceiveDirectMessage(SimChatMessage {
            from_agent: String::from("sys"),
            payload: String::from(msg),
            timestamp: current_timestamp(),
        }))
        .expect("send fail");
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn responds_to_join_event() {
        let (s, _r) = crossbeam_channel::unbounded();
        let mut state = Lib3hSimChatState::new();
        let chat_event = ChatEvent::Join {
            channel_id: String::from("some-channel-id"),
            agent_id: String::from("some-agent-id"),
        };

        let response = handle_chat_event(chat_event, &mut state, s);
        assert_eq!(
            response.unwrap().event,
            ClientToLib3h::JoinSpace(SpaceData {
                space_address: channel_address_from_string(&String::from("some-channel-id"))
                    .unwrap(),
                agent_id: Address::from("some-agent-id"),
                request_id: String::from("")
            })
        );
    }

    #[test]
    fn responds_to_part_event() {
        let (s, _r) = crossbeam_channel::unbounded();
        let mut state = Lib3hSimChatState::new();
        let space_address = channel_address_from_string(&String::from("some-channel-id")).unwrap();
        let space_data = SpaceData {
            space_address: space_address.clone(),
            agent_id: Address::from("some-agent-id"),
            request_id: String::from(""),
        };
        // insert the space to part in the state
        state.spaces.insert(space_address, space_data.clone());
        let chat_event = ChatEvent::Part {
            channel_id: String::from("some-channel-id"),
        };

        let response = handle_chat_event(chat_event, &mut state, s);
        assert_eq!(
            response.unwrap().event,
            ClientToLib3h::LeaveSpace(space_data)
        );
    }

    #[test]
    fn responds_to_send_direct_message() {
        let (s, _r) = crossbeam_channel::unbounded();
        let mut state = Lib3hSimChatState::new();

        let payload = String::from("a message");
        let to_agent_id = String::from("receiver");

        let chat_event = ChatEvent::SendDirectMessage {
            to_agent: to_agent_id.clone(),
            payload: String::from("a message"),
        };

        // set up the state with a current space with an agent it
        state.current_space = Some(SpaceData {
            agent_id: Address::from("sender"),
            request_id: String::from(""),
            space_address: Address::from("some-space"),
        });

        let response = handle_chat_event(chat_event, &mut state, s);
        assert_eq!(
            response.unwrap().event,
            ClientToLib3h::SendDirectMessage(DirectMessageData {
                request_id: String::from(""),
                content: payload.into(),
                to_agent_id: Address::from(to_agent_id),
                from_agent_id: Address::from("sender"),
                space_address: Address::from("some-space")
            })
        );
    }
}

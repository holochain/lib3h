use super::{
    channel_address_from_string, current_timeanchor, current_timestamp, Lib3hSimChatState,
};
use crate::simchat::{ChatEvent, ChatSender, ChatTuple, MessageList, SimChatMessage};
use lib3h::error::Lib3hError;
use lib3h_protocol::{
    data_types::*,
    protocol::{ClientToLib3h, ClientToLib3hResponse, Lib3hToClient, Lib3hToClientResponse},
    Address,
};
use lib3h_tracing::{Lib3hSpan, SpanWrap};
use lib3h_zombie_actor::{GhostCallbackData::Response, GhostCanTrack, GhostContextEndpoint};

pub fn handle_chat_event(
    chat_tuple: ChatTuple,
    state: &mut Lib3hSimChatState,
    parent_endpoint: &mut GhostContextEndpoint<
        (),
        ClientToLib3h,
        ClientToLib3hResponse,
        Lib3hToClient,
        Lib3hToClientResponse,
        Lib3hError,
    >,
    chat_event_sender: ChatSender,
) {
    let SpanWrap {
        data: chat_event,
        span,
    } = chat_tuple;
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
            parent_endpoint
                .request(
                    span.child("ChatEvent::Join"),
                    ClientToLib3h::JoinSpace(space_data.clone()),
                    Box::new(move |_, callback_data| {
                        // println!("chat received response from engine: {:?}", callback_data);
                        if let Response(Ok(_payload)) = callback_data {
                            let new_event = ChatEvent::JoinSuccess {
                                channel_id: channel_id.clone(),
                                space_data: space_data.clone(),
                            };
                            chat_event_sender
                                .send(span.follower("ChatEvent::Join").wrap(new_event))
                                .unwrap();
                        }
                        Ok(())
                    }),
                )
                .unwrap();
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
                span.child("send_sys_message"),
            );
        }

        ChatEvent::Part(channel_id) => {
            if let Some(space_data) = state.current_space.clone() {
                parent_endpoint
                    .request(
                        span.child("ChatEvent::Part"),
                        ClientToLib3h::LeaveSpace(space_data.to_owned()),
                        Box::new(move |_, callback_data| {
                            // println!("chat received response from engine: {:?}", callback_data);
                            if let Response(Ok(_payload)) = callback_data {
                                chat_event_sender
                                    .send(
                                        span.follower("ChatEvent::Part")
                                            .wrap(ChatEvent::PartSuccess(channel_id.clone())),
                                    )
                                    .unwrap();
                            }
                            Ok(())
                        }),
                    )
                    .unwrap();
            } else {
                send_sys_message(
                    chat_event_sender,
                    &"No channel to leave".to_string(),
                    span.child("send_sys_message"),
                );
            }
        }

        ChatEvent::PartSuccess(channel_id) => {
            state.current_space = None;
            state
                .spaces
                .remove(&channel_address_from_string(&channel_id).unwrap());
            send_sys_message(
                chat_event_sender,
                &format!("You left the channel: {}", channel_id).to_string(),
                span.child("send_sys_message"),
            );
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
                parent_endpoint
                    .request(
                        span.child("ChatEvent::SendDirectMessage"),
                        ClientToLib3h::SendDirectMessage(direct_message_data),
                        Box::new(|_, _callback_data| {
                            // println!("chat received response from engine: {:?}", callback_data);
                            // TODO: Track delivered state of message
                            Ok(())
                        }),
                    )
                    .map_err(|e| {
                        send_sys_message(
                            chat_event_sender,
                            &format!("Lib3h returned error: {}", e).to_string(),
                            span.child("send_sys_message"),
                        );
                    })
                    .ok();
            } else {
                send_sys_message(
                    chat_event_sender,
                    &"Must join a channel before sending a message".to_string(),
                    span.child("send_sys_message"),
                );
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

                parent_endpoint
                    .request(
                        span.child("ChatEvent::SendChannelMessage"),
                        ClientToLib3h::PublishEntry(provided_entry_data),
                        Box::new(|_, _callback_data| {
                            // println!("chat received response from engine: {:?}", callback_data);
                            // TODO: Track delivered state of message
                            Ok(())
                        }),
                    )
                    .unwrap();
            } else {
                send_sys_message(
                    chat_event_sender,
                    &"Must join a channel before sending a message".to_string(),
                    span.child("send_sys_message"),
                );
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
                parent_endpoint
                    .request(
                        span.child("ChatEvent::QueryChannelMessages"),
                        ClientToLib3h::QueryEntry(query_entry_data),
                        Box::new(move |_, callback_data| {
                            if let Response(Ok(ClientToLib3hResponse::QueryEntryResult(
                                query_result,
                            ))) = callback_data
                            {
                                let messages = MessageList::from_opaque(query_result.query_result);
                                for message in messages.0 {
                                    // Only emit ReceiveChannelMessage once per time seeing a message
                                    if !displayed_channel_messages.contains(&message.address()) {
                                        chat_event_sender
                                            .send(
                                                span.follower(
                                                    "send: ChatEvent::ReceiveChannelMessage",
                                                )
                                                .wrap(ChatEvent::ReceiveChannelMessage(message)),
                                            )
                                            .ok();
                                    }
                                }
                            }
                            Ok(())
                        }),
                    )
                    .unwrap();
            }
        }

        ChatEvent::ReceiveChannelMessage(message) => {
            state.displayed_channel_messages.push(message.address());
        }

        ChatEvent::Connected => {
            state.connected = true;
        }

        _ => {}
    }
}

fn send_sys_message(sender: ChatSender, msg: &String, span: Lib3hSpan) {
    let event = ChatEvent::ReceiveDirectMessage(SimChatMessage {
        from_agent: String::from("sys"),
        payload: String::from(msg),
        timestamp: current_timestamp(),
    });
    sender.send(span.wrap(event)).expect("send fail");
}

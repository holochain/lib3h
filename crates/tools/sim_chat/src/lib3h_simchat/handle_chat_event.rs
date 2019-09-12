
use super::current_timestamp;
use super::Lib3hSimChatState;
use super::channel_address_from_string;
use crate::simchat::{ChatEvent, SimChatMessage};
use lib3h::error::Lib3hError;
use lib3h_ghost_actor::{
    GhostCallbackData::Response, GhostCanTrack, GhostContextEndpoint,
};
use lib3h_protocol::{
    data_types::{DirectMessageData, SpaceData},
    protocol::{ClientToLib3h, ClientToLib3hResponse, Lib3hToClient, Lib3hToClientResponse},
    Address,
};
use lib3h_tracing::TestTrace;

pub fn handle_chat_event(
	chat_event: ChatEvent,
	state: &mut Lib3hSimChatState,
	parent_endpoint: &mut GhostContextEndpoint<(), TestTrace, ClientToLib3h, ClientToLib3hResponse, Lib3hToClient, Lib3hToClientResponse, Lib3hError>,
	chat_event_sender: crossbeam_channel::Sender<ChatEvent>,
) {
	match chat_event {
        ChatEvent::Join {
            channel_id,
            agent_id,
        } => {
            let space_address =
                channel_address_from_string(&channel_id).unwrap();
            let space_data = SpaceData {
                agent_id: Address::from(agent_id),
                request_id: "".to_string(),
                space_address,
            };
            parent_endpoint
                .request(
                    TestTrace::new(),
                    ClientToLib3h::JoinSpace(space_data.clone()),
                    Box::new(move |_, callback_data| {
                        println!(
                            "chat received response from engine: {:?}",
                            callback_data
                        );
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
                )
                .unwrap();
        }

        ChatEvent::JoinSuccess {
            space_data,
            channel_id,
        } => {
            state.spaces.insert(channel_address_from_string(&channel_id).unwrap(), space_data.clone());
            state.current_space = Some(space_data);
            send_sys_message(
                chat_event_sender,
                &format!("Joined channel: {}", channel_id),
            );
        }

        ChatEvent::Part(channel_id) => {
            if let Some(space_data) = state.current_space.clone() {
                parent_endpoint
                    .request(
                        TestTrace::new(),
                        ClientToLib3h::LeaveSpace(space_data.to_owned()),
                        Box::new(move |_, callback_data| {
                            println!(
                                "chat received response from engine: {:?}",
                                callback_data
                            );
                            if let Response(Ok(_payload)) = callback_data {
                                chat_event_sender
                                    .send(ChatEvent::PartSuccess(
                                        channel_id.clone(),
                                    ))
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
                );
            }
        }

        ChatEvent::PartSuccess(channel_id) => {
            state.current_space = None;
            state.spaces.remove(&channel_address_from_string(&channel_id).unwrap());
            send_sys_message(
                chat_event_sender,
                &"Left channel".to_string(),
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
                        TestTrace::new(),
                        ClientToLib3h::SendDirectMessage(direct_message_data),
                        Box::new(|_, callback_data| {
                            println!(
                                "chat received response from engine: {:?}",
                                callback_data
                            );
                            // TODO: Track delivered state of message
                            Ok(())
                        }),
                    )
                    .unwrap();
            } else {
                send_sys_message(
                    chat_event_sender,
                    &"Must join a channel before sending a message".to_string(),
                );
            }
        },

        // ChatEvent::SendChannelMessage{..} => {

        // }

        _ => {}
    }
}

fn send_sys_message(sender: crossbeam_channel::Sender<ChatEvent>, msg: &String) {
    sender
        .send(ChatEvent::ReceiveDirectMessage(SimChatMessage{
            from_agent: String::from("sys"),
            payload: String::from(msg),
            timestamp: current_timestamp(),
        }))
        .expect("send fail");
}

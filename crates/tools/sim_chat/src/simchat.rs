use lib3h_protocol::data_types::SpaceData;

#[derive(Debug, Clone, PartialEq)]
pub struct SimChatMessage {
    pub from_agent: String,
    pub payload: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChatEvent {
    SendDirectMessage {
        to_agent: String,
        payload: String,
    },
    ReceiveDirectMessage(SimChatMessage),
    Join {
        channel_id: String,
        agent_id: String,
    },
    JoinSuccess {
        channel_id: String,
        space_data: SpaceData,
    },
    RetrieveChannelMessages { // retrieve any messages found within this time box
        start_time: u64,
        end_time: u64,
    },
    SendChannelMessage {
        payload: String,
    },
    ReceiveChannelMessage(SimChatMessage),
    Part(String),
    PartSuccess(String),
    Disconnected,
}

pub trait SimChat {
    fn send(&mut self, event: ChatEvent);
}

use lib3h_protocol::data_types::SpaceData;

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
    Part(String),
    PartSuccess(String),
    Disconnected,
} 

pub trait SimChat {
	fn send(&mut self, event: ChatEvent);
}

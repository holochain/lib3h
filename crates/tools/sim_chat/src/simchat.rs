use lib3h_protocol::data_types::{Opaque, SpaceData};
use serde_json::{from_slice, to_vec};
use serde_derive::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimChatMessage {
    pub from_agent: String,
    pub payload: String,
    pub timestamp: u64,
}

// TODO - dedup this
impl SimChatMessage {
    pub fn from_opaque(o: Opaque) -> Self {
        from_slice(&o.as_bytes()).unwrap()
    }

    // pub fn to_opaque(self) -> Opaque {
    //     to_vec(&self).expect("Could not serialize message").into()
    // }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageList(pub Vec<SimChatMessage>);

impl MessageList {
    // pub fn from_opaque(o: Opaque) -> Self {
    //     from_slice(&o.as_bytes()).unwrap()
    // }

    pub fn to_opaque(self) -> Opaque {
        to_vec(&self)
            .expect("Could not serialize message list")
            .into()
    }
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
    // RetrieveChannelMessages { // retrieve any messages found within this time box
    //     start_time: u64,
    //     end_time: u64,
    // },
    // SendChannelMessage {
    //     payload: String,
    // },
    // ReceiveChannelMessage(SimChatMessage),
    Part(String),
    PartSuccess(String),
    Disconnected,
}

pub trait SimChat {
    fn send(&mut self, event: ChatEvent);
}

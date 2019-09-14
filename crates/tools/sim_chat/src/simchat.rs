use serde::de::DeserializeOwned;
use serde::{Serialize};
use lib3h_protocol::data_types::{Opaque, SpaceData};
use serde_json::{from_slice, to_vec};
use serde_derive::{Serialize, Deserialize};
use lib3h_protocol::Address;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct SimChatMessage {
    pub from_agent: String,
    pub payload: String,
    pub timestamp: u64,
}

pub trait OpaqueConvertable: Sized + Serialize + DeserializeOwned {
    fn from_opaque(o: Opaque) -> Self {
        from_slice(&o.as_bytes()).unwrap()
    }

    fn to_opaque(&self) -> Opaque {
        to_vec(self).expect("Could not serialize message").into()
    }
}

impl OpaqueConvertable for SimChatMessage {}

impl SimChatMessage {
    pub fn address(&self) -> Address {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        Address::from(hasher.finish().to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageList(pub Vec<SimChatMessage>);

impl OpaqueConvertable for MessageList {}

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
    QueryChannelMessages { // retrieve any messages found within this time box
        start_time: u64,
        end_time: u64,
    },
    SendChannelMessage {
        payload: String,
    },
    ReceiveChannelMessage(SimChatMessage),
    Part(String),
    PartSuccess(String),
    Connected,
    Disconnected,
}

pub trait SimChat {
    fn send(&mut self, event: ChatEvent);
}

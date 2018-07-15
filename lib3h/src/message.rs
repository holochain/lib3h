use error;
use rmp_serde;
use libsodacon::net::endpoint::Endpoint;
use std::collections::{hash_map, HashMap};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DiscoveryReq {
    pub discover: HashMap<Vec<u8>, Vec<Endpoint>>,
}

impl DiscoveryReq {
    pub fn new (discover: HashMap<Vec<u8>, Vec<Endpoint>>) -> Self {
        DiscoveryReq {
            discover: discover,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UserMessage {
    pub data: Vec<u8>,
}

impl UserMessage {
    pub fn new (data: Vec<u8>) -> Self {
        UserMessage {
            data: data
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Message {
    DiscoveryReq(Box<DiscoveryReq>),
    UserMessage(Box<UserMessage>),
}

pub fn compile(
    message: Message,
) -> error::Result<Vec<u8>> {
    Ok(rmp_serde::to_vec(&message)?)
}

pub fn parse(message: Vec<u8>) -> error::Result<Message> {
    Ok(rmp_serde::from_slice(&message)?)
}

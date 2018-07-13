use error;
use libsodacrypt;
use net::http;
use rmp_serde;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_millis () -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
    since_the_epoch.as_secs() * 1000 +
        since_the_epoch.subsec_nanos() as u64 / 1_000_000
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InitialHandshakeRes {
    pub session_id: String,
    pub node_id: Vec<u8>,
    pub eph_pub: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PingReq {
    pub sent_time: u64,
}

impl PingReq {
    pub fn new () -> Self {
        PingReq {
            sent_time: get_millis(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PingRes {
    pub origin_time: u64,
    pub response_time: u64,
}

impl PingRes {
    pub fn new (origin_time: u64) -> Self {
        PingRes {
            origin_time: origin_time,
            response_time: get_millis(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UserMessage {
    pub data: Vec<u8>,
}

impl UserMessage {
    pub fn new (data: &[u8]) -> Self {
        UserMessage {
            data: data.to_vec()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Message {
    PingReq(Box<PingReq>),
    PingRes(Box<PingRes>),
    UserMessage(Box<UserMessage>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct MsgWrap (Vec<u8>, Vec<u8>);

pub fn compile(
    session_id: &str,
    sub_messages: &Vec<Message>,
    rtype: http::RequestType,
    psk: &[u8],
) -> error::Result<Vec<u8>> {
    let msg = rmp_serde::to_vec(sub_messages)?;

    let (nonce, msg) = libsodacrypt::sym::enc(&msg, psk)?;
    let msg = rmp_serde::to_vec(&MsgWrap(nonce, msg))?;

    let mut req_out = http::Request::new(rtype);
    req_out.method = "POST".to_string();
    req_out.path = format!("/{}", session_id);
    req_out.code = "200".to_string();
    req_out.status = "OK".to_string();
    req_out.headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string(),
    );
    req_out.body = msg;

    let msg = req_out.generate();

    Ok(msg)
}

pub fn parse(message: &[u8], psk: &[u8]) -> error::Result<Vec<Message>> {
    let message: MsgWrap = rmp_serde::from_slice(message)?;
    let message = libsodacrypt::sym::dec(&message.1, &message.0, psk)?;
    let message: Vec<Message> = rmp_serde::from_slice(&message)?;
    Ok(message)
}

use error;
use hex;
use net::http;
use rmp_serde;
use std::time::{SystemTime, UNIX_EPOCH};

fn get_millis () -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
    since_the_epoch.as_secs() * 1000 +
        since_the_epoch.subsec_nanos() as u64 / 1_000_000
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
pub enum Message {
    PingReq(Box<PingReq>),
    PingRes(Box<PingRes>),
}

pub fn compile(
    node_id: &Vec<u8>,
    sub_messages: &Vec<Message>,
    rtype: http::RequestType,
) -> error::Result<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();

    let mut msg = rmp_serde::to_vec(sub_messages)?;

    out.append(&mut msg);

    let mut req_out = http::Request::new(rtype);
    req_out.method = "POST".to_string();
    req_out.path = format!("/{}", hex::encode(node_id));
    req_out.code = "200".to_string();
    req_out.status = "OK".to_string();
    req_out.headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string(),
    );
    req_out.body = out;

    let out = req_out.generate();

    Ok(out)
}

pub fn parse(message: &[u8]) -> error::Result<Vec<Message>> {
    let out: Vec<Message> = rmp_serde::from_slice(message)?;
    Ok(out)
}

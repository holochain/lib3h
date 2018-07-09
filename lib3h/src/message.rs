use crypto;
use error;
use http;
use netinfo;
use rand;
use rmp_serde;
use std;

use rand::Rng;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReqNetInfoSet {
    pub start_tag: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ResNetInfoSet {
    pub net_info_set: Vec<netinfo::NodeInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Message {
    ReqNetInfoSet(Box<ReqNetInfoSet>),
    ResNetInfoSet(Box<ResNetInfoSet>),
}

pub fn compile(
    sub_messages: &Vec<Message>,
    psk: &[u8],
    rtype: http::RequestType,
) -> error::Result<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    let mut rng = rand::thread_rng();

    let mut garbage: Vec<u8> = Vec::new();
    garbage.resize(1024, 0);
    rng.fill(garbage.as_mut_slice());
    out.append(&mut garbage);

    let mut msg = rmp_serde::to_vec(sub_messages)?;

    out.append(&mut msg);

    let remain = 1024 + 4096 - out.len();
    if (remain > 0) {
        let mut pad: Vec<u8> = Vec::new();
        pad.resize(remain, 0);
        rng.fill(pad.as_mut_slice());
        out.append(&mut pad);
    }

    let out = crypto::aes::enc(&out, psk)?;

    let mut req_out = http::Request::new(rtype);
    req_out.method = "GET".to_string();
    req_out.path = "/".to_string();
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

pub fn parse(message: &[u8], psk: &[u8]) -> error::Result<Vec<Message>> {
    let mut out = crypto::aes::dec(message, psk)?;
    out.drain(..1024);
    let out: Vec<Message> = rmp_serde::from_slice(&out)?;
    Ok(out)
}

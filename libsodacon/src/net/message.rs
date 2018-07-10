use error;
use net::http;
use rmp_serde;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InitialHandshakeRes {
    pub node_id: Vec<u8>,
    pub eph_pub: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Message {
    InitialHandshakeRes(Box<InitialHandshakeRes>),
}

pub fn compile(
    sub_messages: &Vec<Message>,
    rtype: http::RequestType,
) -> error::Result<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();

    let mut msg = rmp_serde::to_vec(sub_messages)?;

    out.append(&mut msg);

    let mut req_out = http::Request::new(rtype);
    req_out.method = "POST".to_string();
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

pub fn parse(message: &[u8]) -> error::Result<Vec<Message>> {
    let out: Vec<Message> = rmp_serde::from_slice(message)?;
    Ok(out)
}

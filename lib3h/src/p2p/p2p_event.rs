#[derive(Debug, PartialEq, Clone)]
pub enum P2pEvent {
    HandleRequest(HandleRequestData),
    HandlePublish(HandlePublishData),
    PeerConnected(String),
    PeerDisconnected(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct HandleRequestData {
    pub from_peer_address: String,
    pub msg_id: String,
    pub data: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct HandlePublishData {
    pub from_peer_address: String,
    pub data: Vec<u8>,
}
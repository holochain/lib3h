use error;
use net::endpoint::Endpoint;

#[derive(Debug)]
pub enum ServerEvent {
    OnError(error::Error),
    OnListening(Endpoint),
    OnConnection(Vec<u8>, Endpoint),
    OnDataReceived(Vec<u8>, Vec<u8>),
    OnClose(),
}

#[derive(Debug)]
pub enum ClientEvent {
    OnError(error::Error),
    OnConnected(Vec<u8>, Endpoint),
    OnDataReceived(Vec<u8>, Vec<u8>),
    OnClose(),
}

#[derive(Debug)]
pub enum Event {
    OnError(error::Error),
    OnServerEvent(ServerEvent),
    OnClientEvent(ClientEvent),
}

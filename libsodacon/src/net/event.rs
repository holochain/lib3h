use errors::*;
use net::endpoint::Endpoint;

#[derive(Debug)]
pub enum ServerEvent {
    OnError(Error),
    OnListening(Endpoint),
    OnConnection(Vec<u8>, Endpoint),
    OnDataReceived(Vec<u8>, Vec<u8>),
    OnClose(),
}

#[derive(Debug)]
pub enum ClientEvent {
    OnError(Error),
    OnConnected(Vec<u8>, Endpoint),
    OnDataReceived(Vec<u8>, Vec<u8>),
    OnClose(),
}

#[derive(Debug)]
pub enum Event {
    OnError(Error),
    OnServerEvent(ServerEvent),
    OnClientEvent(ClientEvent),
}

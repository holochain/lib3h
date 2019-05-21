use holochain_lib3h_protocol::{Lib3hResult, Address, DidWork};
use crate::p2p::p2p_event::P2pEvent;

pub trait P2p {
    fn init(&self);
    fn id(&self) -> Address;
    fn advertise(&self) -> String;
    fn transportConnect(&self, url: String) -> Lib3hResult<()>;
    fn close(&self, peer: Address) -> Lib3hResult<()>;

    fn publish_reliable(&self, peerList: Vec<Address>, data: Vec<u8>) -> Lib3hResult<()>;
    fn publish_unreliable(&self, peerList: Vec<Address>, data: Vec<u8>) -> Lib3hResult<()>;

    fn request_reliable(&self, peerList: Vec<Address>, data: Vec<u8>) -> Lib3hResult<()>;
    fn respond_reliable(&self, msg_id: String, data: Vec<u8>) -> Lib3hResult<()>;

    fn post(&mut self, evt: P2pEvent) -> Lib3hResult<()>;
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<P2pEvent>)>;
}

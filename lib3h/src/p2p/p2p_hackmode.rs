#![allow(non_snake_case)]

use crate::{
    dht::{dht_event::DhtEvent, dht_trait::Dht, rrdht::rrdht},
    p2p::{p2p_event::P2pEvent, p2p_trait::P2p},
    transport::{Transport, TransportEvent},
    transport_wss::TransportWss,
};
use holochain_lib3h_protocol::{DidWork, Lib3hResult};
use std::collections::VecDeque;

pub struct P2pHackmode {
    transport_connection: TransportWss<std::net::TcpStream>,
    dht: Box<Dht>,
    /// FIFO of P2pEvents received from the transport_connection
    inbox: VecDeque<P2pEvent>,
}

impl P2pHackmode {
    pub fn new() -> Self {
        P2pHackmode {
            transport_connection: TransportWss::with_std_tcp_stream(),
            dht: Box::new(rrdht::new()),
            inbox: VecDeque::new(),
        }
    }
}

impl P2p for P2pHackmode {
    // -- Getters -- //

    fn id(&self) -> String {
        // FIXME
        "FIXME".to_string()
    }
    fn advertise(&self) -> String {
        // FIXME
        "FIXME".to_string()
    }

    // -- Lifecycle -- //

    fn connect(&mut self, ipc_uri: String) -> Lib3hResult<()> {
        // FIXME
        let _transport_id = self.transport_connection.wait_connect(&ipc_uri)?;
        Ok(())
    }
    fn close(&self, _peer: String) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }

    // -- Comms -- //

    fn publish_reliable(&self, _peer_list: Vec<String>, _data: Vec<u8>) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn publish_unreliable(&self, _peer_list: Vec<String>, _data: Vec<u8>) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }

    fn request_reliable(&self, _peer_list: Vec<String>, _data: Vec<u8>) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn respond_reliable(&self, _msg_id: String, _data: Vec<u8>) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }

    // -- Processing -- //

    fn post(&mut self, evt: P2pEvent) -> Lib3hResult<()> {
        self.inbox.push_back(evt);
        Ok(())
    }

    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<P2pEvent>)> {
        // Process the transport connection
        let (did_work, transport_event_list) = self.transport_connection.process()?;
        if did_work {
            for evt in transport_event_list {
                let p2p_output = self.serve_TransportEvent(evt)?;
                // Add p2p events output to self's inbox
                for p2p_evt in p2p_output {
                    self.post(p2p_evt)?;
                }
            }
        }
        // Process the transport dht
        let (did_work, dht_event_list) = self.dht.process()?;
        if did_work {
            for evt in dht_event_list {
                let p2p_output = self.serve_DhtEvent(evt)?;
                // Add p2p events output to self's inbox
                for p2p_evt in p2p_output {
                    self.post(p2p_evt)?;
                }
            }
        }
        // Process the P2pEvents
        let (did_work, outbox) = self.process_inbox()?;
        // Done
        Ok((did_work, outbox))
    }
}

impl P2pHackmode {
    /// Progressively serve every event received in inbox
    fn process_inbox(&mut self) -> Lib3hResult<(DidWork, Vec<P2pEvent>)> {
        let mut outbox = Vec::new();
        let mut did_work = false;
        loop {
            let evt = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            let (success, mut output) = self.serve_P2pEvent(evt)?;
            if success {
                did_work = success;
            }
            outbox.append(&mut output);
        }
        Ok((did_work, outbox))
    }

    /// Process a P2pEvent sent to us.
    /// Return a list of P2pEvents to send to others.
    fn serve_P2pEvent(&self, evt: P2pEvent) -> Lib3hResult<(DidWork, Vec<P2pEvent>)> {
        println!("(log.d) >>> '(P2pHackmode)' recv: {:?}", evt);
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Note: use same order as the enum
        match evt {
            P2pEvent::HandleRequest(_data) => {
                // FIXME
            }
            P2pEvent::HandlePublish(_data) => {
                // FIXME
            }
            P2pEvent::PeerConnected(_peer_address) => {
                // FIXME
            }
            P2pEvent::PeerDisconnected(_peer_address) => {
                // FIXME
            }
        };
        Ok((did_work, outbox))
    }

    /// Serve a transportEvent sent to us.
    /// Return a list of P2pEvents for us to process.
    // FIXME
    fn serve_TransportEvent(&mut self, evt: TransportEvent) -> Lib3hResult<Vec<P2pEvent>> {
        let mut outbox = Vec::new();
        match evt {
            TransportEvent::TransportError(id, _e) => {
                // FIXME
                //self.log.e(&format!("ipc ws error {:?}", e));
                self.transport_connection.close(id)?;
                //let _transport_id = self.transport_connection.wait_connect(&self.ipc_uri)?;
            }
            TransportEvent::Connect(_id) => {
                // FIXME
                // don't need to do anything here
            }
            TransportEvent::Close(id) => {
                // FIXME
                //self.log.e("ipc ws closed");
                self.transport_connection.close(id)?;
                //let _transport_id = self.wss_socket.wait_connect(&self.ipc_uri)?;
            }
            TransportEvent::Message(_id, _msg) => {
                // FIXME
            }
        };
        Ok(outbox)
    }

    /// Serve a DhtEvent sent to us.
    /// Return a list of P2pEvents for us to process.
    // FIXME
    fn serve_DhtEvent(&mut self, evt: DhtEvent) -> Lib3hResult<Vec<P2pEvent>> {
        let mut outbox = Vec::new();
        match evt {
            DhtEvent::RemoteGossipBundle(_data) => {
                // FIXME
            }
            DhtEvent::GossipTo(_data) => {
                // FIXME
            }
            DhtEvent::UnreliableGossipTo(_data) => {
                // FIXME
            }
            DhtEvent::PeerHoldRequest(_data) => {
                // FIXME
            }
            DhtEvent::PeerTimedOut(_peer_address) => {
                // FIXME
            }
            DhtEvent::DataHoldRequest(_data) => {
                // FIXME
            }
            DhtEvent::DataFetch(_data) => {
                // FIXME
            }
            DhtEvent::DataFetchResponse(_data) => {
                // FIXME
            }
            DhtEvent::DataPrune(_peer_address) => {
                // FIXME
            }
        }
        Ok(outbox)
    }
}

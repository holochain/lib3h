use std::collections::VecDeque;
use holochain_lib3h_protocol::{Lib3hResult, Address, DidWork};
use crate::{
    dht::{
        dht_trait::Dht,
        rrdht::rrdht,
        dht_event::DhtEvent,
    },
    p2p::{
        p2p_trait::P2p,
        p2p_event::P2pEvent,
    },
};

// tmp
use crate::transport::*;

pub struct P2pHackmode {
    transport_connection: TransportWss<std::net::TcpStream>,
    dht: Box<Dht>,
    /// FIFO of messages received from ???
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
    fn init(&self) {
        // FIXME
    }
    fn id(&self) -> Address {
        // FIXME
        "FIXME".to_string()
    }
    fn advertise(&self) -> String {
        // FIXME
        "FIXME".to_string()
    }

    fn transportConnect(&self, ipc_uri: String) -> Lib3hResult<()> {
        // FIXME
        let transport_id = self.transport_connection.wait_connect(&ipc_uri)?;
        Ok(())
    }
    fn close(&self, _peer: Address) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }

    fn publish_reliable(&self, _peerList: Vec<Address>, _data: Vec<u8>) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn publish_unreliable(&self, _peerList: Vec<Address>, _data: Vec<u8>) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }

    fn request_reliable(&self, _peerList: Vec<Address>, _data: Vec<u8>) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn respond_reliable(&self, _msg_id: String, _data: Vec<u8>) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }

    fn post(&mut self, evt: P2pEvent) -> Lib3hResult<()> {
        self.inbox.push_back(evt);
        Ok(())
    }

    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<P2pEvent>)> {
        // Process the transport connection
        let (did_work, transport_event_list) = self.transport_connection.poll()?;
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
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Progressively serve every event received in inbox
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
}

impl P2pHackmode {
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
            },
            P2pEvent::HandlePublish(_data) => {
                // FIXME
            },
            P2pEvent::PeerConnected(_peer_address) => {
                // FIXME
            },
            P2pEvent::PeerDisconnected(_peer_address) => {
                // FIXME
            },
        };
        Ok((did_work, outbox))
    }

    /// Serve a transportEvent sent to us
    /// Return a list of P2pEvent to send to owner?
    // FIXME
    fn serve_TransportEvent(&mut self, evt: TransportEvent) -> Lib3hResult<Vec<P2pEvent>> {
        let mut outbox = Vec::new();
        match evt {
            TransportEvent::TransportError(_id, e) => {
                // FIXME
                //self.log.e(&format!("ipc ws error {:?}", e));
                self.transport_connection.close(self.transport_id.clone())?;
                self.transport_id = self.transport_connection.wait_connect(&self.ipc_uri)?;
            }
            TransportEvent::Connect(_id) => {
                // FIXME
                // don't need to do anything here
            }
            TransportEvent::Close(_id) => {
                // FIXME
                //self.log.e("ipc ws closed");
                self.transport_connection.close(self.transport_id.clone())?;
                self.transport_id = self.wss_socket.wait_connect(&self.ipc_uri)?;
            }
            TransportEvent::Message(_id, msg) => {
                // FIXME
            }
        };
        Ok(outbox)
    }

    /// Serve a DhtEvent sent to us
    /// Return a list of P2pEvent to send to owner?
    // FIXME
    fn serve_DhtEvent(&mut self, evt: DhtEvent) -> Lib3hResult<Vec<P2pEvent>> {
        let mut outbox = Vec::new();
        match evt {
            DhtEvent::remoteGossipBundle(_data) => {
                // FIXME
            },
            DhtEvent::GossipTo(_data) => {
                // FIXME
            },
            DhtEvent::UnreliableGossipTo(_data) => {
                // FIXME
            },
            DhtEvent::PeerHoldRequest(_data) => {
                // FIXME
            },
            DhtEvent::PeerTimedOut(_peer_address) => {
                // FIXME
            },
            DhtEvent::DataHoldRequest(_data) => {
                // FIXME
            },
            DhtEvent::DataFetch(_data) => {
                // FIXME
            },
            DhtEvent::DataFetchResponse(_data) => {
                // FIXME
            },
            DhtEvent::DataPrune(_peer_address) => {
                // FIXME
            },
        }
        Ok(outbox)
    }
}
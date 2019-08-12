use crate::transport::{
    error::TransportResult,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    ConnectionId, ConnectionIdRef, TransportWrapper,
};
use lib3h_protocol::{Address, DidWork};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use url::Url;

#[derive(Serialize, Deserialize, Clone, Debug, PartialOrd, Ord, Hash, PartialEq, Eq)]
struct ChannelSpec {
    space_address: Address,
    agent_id: Address,
}

/// Wraps any transport and adds cryptography
pub struct TransportMultiplex<'mplex> {
    inner_transport: TransportWrapper<'mplex>,
    channels: HashMap<ChannelSpec, Arc<RwLock<TransportUniplex<'mplex>>>>,
}

/// Constructor
impl<'mplex> TransportMultiplex<'mplex> {
    pub fn new(inner_transport: TransportWrapper<'mplex>) -> Self {
        TransportMultiplex {
            inner_transport,
            channels: HashMap::new(),
        }
    }

    /// get a transport channel that will communicate between given space address and agent id pair
    pub fn get_channel(
        &mut self,
        space_address: Address,
        local_agent_id: Address,
    ) -> TransportResult<TransportWrapper<'mplex>> {
        let channel = ChannelSpec {
            space_address,
            agent_id: local_agent_id,
        };
        match self.channels.get(&channel) {
            Some(w) => Ok(TransportWrapper::assume(w.clone())),
            None => {
                let out = Arc::new(RwLock::new(TransportUniplex {
                    inner_transport: self.inner_transport.clone(),
                    channel: channel.clone(),
                    outbox: Vec::new(),
                }));
                self.channels.insert(channel, out.clone());
                Ok(TransportWrapper::assume(out))
            }
        }
    }

    pub fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        let (did_work, evt_list) = self.inner_transport.as_mut().process()?;
        let mut out_events = Vec::new();

        for evt in evt_list {
            match evt {
                TransportEvent::ReceivedData(ref _id, ref payload) => {
                    let mut de = rmp_serde::Deserializer::new(&payload[..]);
                    let maybe_channel_msg: Result<ChannelWrappedIn, rmp_serde::decode::Error> =
                        Deserialize::deserialize(&mut de);
                    if let Ok(msg) = maybe_channel_msg {
                        match self.channels.get_mut(&msg.channel) {
                            Some(c) => {
                                c.write()
                                    .expect("failed to obtain write lock")
                                    .outbox
                                    .push(evt);
                            }
                            None => out_events.push(evt),
                        }
                    } else {
                        out_events.push(evt);
                    }
                }
                _ => out_events.push(evt),
            }
        }

        Ok((did_work, out_events))
    }
}

struct TransportUniplex<'mplex> {
    inner_transport: TransportWrapper<'mplex>,
    channel: ChannelSpec,
    outbox: Vec<TransportEvent>,
}

#[derive(Serialize)]
struct ChannelWrappedOut<'a> {
    channel: &'a ChannelSpec,
    payload: &'a [u8],
}

#[derive(Deserialize, Debug)]
struct ChannelWrappedIn {
    channel: ChannelSpec,
    payload: Vec<u8>,
}

impl<'mplex> TransportUniplex<'mplex> {
    fn wrap_payload(&self, to_address: &Address, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        ChannelWrappedOut {
            channel: &ChannelSpec {
                space_address: self.channel.space_address.clone(),
                agent_id: to_address.clone(),
            },
            payload,
        }
        .serialize(&mut rmp_serde::Serializer::new(&mut out))
        .expect("P2pProtocol::Gossip serialization failed");
        out
    }
}

/// Implement Transport trait by composing inner transport
impl<'mplex> Transport for TransportUniplex<'mplex> {
    fn connect(&mut self, uri: &Url) -> TransportResult<ConnectionId> {
        // TODO XXX - we need to associate the dest agent id with the returned ConnectionId
        self.inner_transport.as_mut().connect(&uri)
    }
    fn close(&mut self, id: &ConnectionIdRef) -> TransportResult<()> {
        self.inner_transport.as_mut().close(id)
    }

    fn close_all(&mut self) -> TransportResult<()> {
        self.inner_transport.as_mut().close_all()
    }

    fn send(&mut self, id_list: &[&ConnectionIdRef], payload: &[u8]) -> TransportResult<()> {
        let to_address: Address = "".to_string().into();
        // TODO XXX - we need to look up the agent address for this con id
        self.inner_transport
            .as_mut()
            .send(id_list, &self.wrap_payload(&to_address, payload))
    }

    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        let to_address: Address = "".to_string().into();
        self.inner_transport
            .as_mut()
            .send_all(&self.wrap_payload(&to_address, payload))
    }

    fn bind(&mut self, url: &Url) -> TransportResult<Url> {
        self.inner_transport.as_mut().bind(url)
    }

    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.inner_transport.as_mut().post(command)
    }

    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        Ok((!self.outbox.is_empty(), self.outbox.drain(..).collect()))
    }

    fn connection_id_list(&self) -> TransportResult<Vec<ConnectionId>> {
        self.inner_transport.as_ref().connection_id_list()
    }

    fn get_uri(&self, id: &ConnectionIdRef) -> Option<Url> {
        self.inner_transport.as_ref().get_uri(id)
    }
}

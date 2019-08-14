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

/// our hashmap uses this struct for keys
#[derive(Serialize, Deserialize, Clone, Debug, PartialOrd, Ord, Hash, PartialEq, Eq)]
struct LocalChannelSpec {
    space_address: Address,
    local_agent_id: Address,
}

/// Split a transport into "channels" -- sub-transports that act on behalf of
/// a single space_address / local agent combination
pub struct TransportMultiplex<'mplex> {
    inner_transport: TransportWrapper<'mplex>,
    channels: HashMap<LocalChannelSpec, Arc<RwLock<TransportUniplex<'mplex>>>>,
}

/// Constructor
impl<'mplex> TransportMultiplex<'mplex> {
    /// multiplex a transport by constructing a TransportMultiplex around it
    /// then calling "get_channel" to fetch the sub-transports
    pub fn new(inner_transport: TransportWrapper<'mplex>) -> Self {
        TransportMultiplex {
            inner_transport,
            channels: HashMap::new(),
        }
    }

    /// get a transport channel that will communicate between given space_address / local_agent_id
    /// pair and any remote agent ids in the same space_address
    pub fn get_channel(
        &mut self,
        space_address: &Address,
        local_agent_id: &Address,
    ) -> TransportResult<TransportWrapper<'mplex>> {
        let channel = LocalChannelSpec {
            space_address: space_address.clone(),
            local_agent_id: local_agent_id.clone(),
        };
        match self.channels.get(&channel) {
            Some(w) => Ok(TransportWrapper::assume(w.clone())),
            None => {
                let out = Arc::new(RwLock::new(TransportUniplex {
                    inner_transport: self.inner_transport.clone(),
                    channel: channel.clone(),
                    outbox: Vec::new(),
                    con_id_to_agent: HashMap::new(),
                }));
                self.channels.insert(channel, out.clone());
                Ok(TransportWrapper::assume(out))
            }
        }
    }

    /// process any incoming messages... messages destined for a sub-channel
    /// are routed to the sub-transports to be seen on their "process" functions
    /// any other messages will be returned here.
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
                        let channel = LocalChannelSpec {
                            space_address: msg.space_address.clone(),
                            local_agent_id: msg.local_agent_id.clone(),
                        };
                        match self.channels.get_mut(&channel) {
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

/// inner private structure representing a sub-transport (channel)
/// this is only exposed out of this function as a trait-object
/// TransportWrapper
struct TransportUniplex<'mplex> {
    inner_transport: TransportWrapper<'mplex>,
    channel: LocalChannelSpec,
    outbox: Vec<TransportEvent>,
    #[allow(dead_code)]
    con_id_to_agent: HashMap<ConnectionId, Address>,
}

#[derive(Serialize)]
struct ChannelWrappedOut<'a> {
    space_address: &'a Address,
    remote_agent_id: &'a Address,
    payload: &'a [u8],
}

#[derive(Deserialize, Debug)]
struct ChannelWrappedIn {
    space_address: Address,
    local_agent_id: Address,
    payload: Vec<u8>,
}

impl<'mplex> TransportUniplex<'mplex> {
    /// when we actually implement the p2p multiplexing protocol
    /// it will register symbolic indexes to channel space_address/agent pairs
    /// to more economically communicate the info.
    /// For now, we're just sending it on every message.
    fn wrap_payload(&self, to_address: &Address, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        ChannelWrappedOut {
            space_address: &self.channel.space_address,
            remote_agent_id: to_address,
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
        let id = self.inner_transport.as_mut().connect(&uri)?;
        /* TODO XXX -
        let prev = self
            .con_id_to_agent
            .insert(id.clone(), uri.path().to_string().into());
        if prev.is_some() {
            return Err(format!("{} already mapped to {:?}", id, prev).into());
        }
        */
        Ok(id)
    }

    fn close(&mut self, id: &ConnectionIdRef) -> TransportResult<()> {
        self.inner_transport.as_mut().close(id)
    }

    fn close_all(&mut self) -> TransportResult<()> {
        self.inner_transport.as_mut().close_all()
    }

    fn send(&mut self, id_list: &[&ConnectionIdRef], payload: &[u8]) -> TransportResult<()> {
        // TODO XXX - we're jamming addresses into the connectionId field
        //            need to actually use connect
        /*
        for id in id_list {
            let to_address = self.con_id_to_agent.get(&id.to_string());
            if to_address.is_none() {
                return Err(format!("no connection for {}", id).into());
            }
            let to_address = to_address.unwrap();
            self.inner_transport
                .as_mut()
                .send(&[id], &self.wrap_payload(&to_address, payload))?;
        }
        */
        // BEGIN HACK
        for id in id_list {
            let address: Address = id.to_string().into();
            self.inner_transport
                .as_mut()
                .send(&[id], &self.wrap_payload(&address, payload))?;
        }
        // END HACK
        Ok(())
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


use std::{
    sync::mpsc,
};

use holochain_lib3h_protocol::{
    Lib3hResult,
    protocol::Lib3hProtocol,
};
use crate::{
    network_engine::NetworkEngine,
    p2p_protocol::P2pProtocol,
};

/// Lib3h's 'mock mode' as a NetworkEngine
pub struct MockEngine {
    /// End-user's network config
    config: serde_json::Value,
    /// Sender to the Worker
    tx: mpsc::Sender<Lib3hProtocol>,
    /// identifier
    name: String,
}

impl MockEngine {
    pub fn new(config: serde_json::Value, tx: mpsc::Sender<Lib3hProtocol>) -> Lib3hResult<Self> {
        Ok(MockEngine {
            config,
            tx,
            name: "FIXME".to_string(),
        })
    }
    /// Received message from p2p network.
    /// -> Process it or forward to local client
    fn receive(&self, p2p_msg: P2pProtocol) -> Lib3hResult<()> {
        println!("(log.d) <<<< '{}' p2p recv: {:?}", self.name.clone(), p2p_msg);
        // Note: use same order as the enum
        match p2p_msg {
            P2pProtocol::DirectMessage => {
                // FIXME
                // For now just send something to local client
                let data = holochain_lib3h_protocol::data_types::DirectMessageData {
                    dna_address: vec![42],
                    request_id: "FIXME".to_string(),
                    to_agent_id: "FIXME".to_string(),
                    from_agent_id: "FIXME".to_string(),
                    content: vec![42],
                };
                self.tx.send(Lib3hProtocol::SendDirectMessageResult(data))?;
            }
            _ => {
                panic ! ("unexpected {:?}", &p2p_msg);
            }
        }
        Ok(())
    }
}

impl NetworkEngine for MockEngine {
    fn run(&self) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn stop(&self) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn terminate(&self) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn advertise(&self) -> String {
        "FIXME".to_string()
    }

    /// process a message sent by our local Client
    fn serve(&self, local_msg: Lib3hProtocol) -> Lib3hResult<()> {
        println!("(log.d) >>>> '{}' recv: {:?}", self.name.clone(), local_msg);
        // Note: use same order as the enum
        match local_msg {
            Lib3hProtocol::SuccessResult(_msg) => {
                // FIXME
            }
            Lib3hProtocol::FailureResult(_msg) => {
                // FIXME
            }
            Lib3hProtocol::TrackDna(_msg) => {
                // FIXME
            }
            Lib3hProtocol::UntrackDna(_msg) => {
                // FIXME
            }
            Lib3hProtocol::SendDirectMessage(_msg) => {
                // FIXME
            }
            Lib3hProtocol::HandleSendDirectMessageResult(_msg) => {
                // FIXME
            }
            Lib3hProtocol::FetchEntry(_msg) => {
                // FIXME
            }
            Lib3hProtocol::HandleFetchEntryResult(_msg) => {
                // FIXME
            }
            Lib3hProtocol::PublishEntry(_msg) => {
                // FIXME
            }
            Lib3hProtocol::FetchMeta(_msg) => {
                // FIXME
            }
            Lib3hProtocol::HandleFetchMetaResult(_msg) => {
                // FIXME
            }
            Lib3hProtocol::PublishMeta(_msg) => {
                // FIXME
            }
            // Our request for the publish_list has returned
            Lib3hProtocol::HandleGetPublishingEntryListResult(_msg) => {
                // FIXME
            }
            // Our request for the hold_list has returned
            Lib3hProtocol::HandleGetHoldingEntryListResult(_msg) => {
                // FIXME
            }
            // Our request for the publish_meta_list has returned
            Lib3hProtocol::HandleGetPublishingMetaListResult(_msg) => {
                // FIXME
            }
            // Our request for the hold_meta_list has returned
            Lib3hProtocol::HandleGetHoldingMetaListResult(_msg) => {
                // FIXME
            }
            _ => {
                panic!("unexpected {:?}", &local_msg);
            }
        }
        Ok(())
    }
}


use std::{
    collections::{hash_map::Entry, HashMap},
    convert::TryFrom,
    sync::{mpsc, Mutex},
};

use holochain_lib3h_protocol::protocol::Lib3hProtocol;
use crate::network_engine::NetworkEngine;
use p2p_protocol::P2pProtocol;

/// Lib3h's 'mock mode' as a NetworkEngine
struct MockEngine {
    /// End-user's network config
    config: serde_json::Value,
    /// Sender to the Worker
    tx: mpsc::Sender<Lib3hProtocol>,
    /// identifier
    name: String,
}

impl MockEngine {
    pub fn new(config: serde_json::Value, tx: mpsc::Sender<Lib3hProtocol>) -> NetResult<Self> {
        Ok(Lib3hMain {
            config,
            tx,
            name: "FIXME",
        })
    }
    /// Received message from p2p network.
    /// -> Process it or forward to local client
    fn receive(&mut self, data: P2pProtocol) -> NetResult<()> {
        println!("(log.d) <<<< '{}' p2p recv: {:?}", self.name.clone(), data);
        // serve only JsonProtocol
        let maybe_json_msg = Lib3hProtocol::try_from(&data);
        if maybe_json_msg.is_err() {
            return Ok(());
        };
        // Note: use same order as the enum
        match maybe_json_msg.as_ref().unwrap() {
            Lib3hProtocol::SendMessageResult(msg) => {
                // FIXME
                self.tx.send(msg)?;
            }
            _ => {
                panic ! ("unexpected {:?}", & maybe_json_msg);
            }
        }
        Ok(())
    }
}

impl NetworkEngine for MockEngine {
    fn run(&mut self) -> NetResult<()> {
        // FIXME
        Ok(())
    }
    fn stop(&mut self) -> NetResult<()> {
        // FIXME
        Ok(())
    }
    fn terminate(&mut self) -> NetResult<()> {
        // FIXME
        Ok(())
    }
    fn advertise(&self) -> String {
        "FIXME".to_string()
    }

    /// process a message sent by our local Client
    fn serve(&mut self, data: Lib3hProtocol) -> NetResult<()> {
        println!("(log.d) >>>> '{}' recv: {:?}", self.name.clone(), data);
        // serve only JsonProtocol
        let maybe_json_msg = JsonProtocol::try_from(&data);
        if maybe_json_msg.is_err() {
            return Ok(());
        };
        // Note: use same order as the enum
        match maybe_json_msg.as_ref().unwrap() {
            Lib3hProtocol::SuccessResult(msg) => {
                // FIXME
            }
            Lib3hProtocol::FailureResult(msg) => {
                // FIXME
            }
            Lib3hProtocol::TrackDna(msg) => {
                // FIXME
            }
            Lib3hProtocol::UntrackDna(msg) => {
                // FIXME
            }
            Lib3hProtocol::SendMessage(msg) => {
                // FIXME
            }
            Lib3hProtocol::HandleSendMessageResult(msg) => {
                // FIXME
            }
            Lib3hProtocol::FetchEntry(msg) => {
                // FIXME
            }
            Lib3hProtocol::HandleFetchEntryResult(msg) => {
                // FIXME
            }
            Lib3hProtocol::PublishEntry(msg) => {
                // FIXME
            }
            Lib3hProtocol::FetchMeta(msg) => {
                // FIXME
            }
            Lib3hProtocol::HandleFetchMetaResult(msg) => {
                // FIXME
            }
            Lib3hProtocol::PublishMeta(msg) => {
                // FIXME
            }
            // Our request for the publish_list has returned
            Lib3hProtocol::HandleGetPublishingEntryListResult(msg) => {
                // FIXME
            }
            // Our request for the hold_list has returned
            Lib3hProtocol::HandleGetHoldingEntryListResult(msg) => {
                // FIXME
            }
            // Our request for the publish_meta_list has returned
            Lib3hProtocol::HandleGetPublishingMetaListResult(msg) => {
                // FIXME
            }
            // Our request for the hold_meta_list has returned
            Lib3hProtocol::HandleGetHoldingMetaListResult(msg) => {
                // FIXME
            }
            _ => {
                panic!("unexpected {:?}", &maybe_json_msg);
            }
        }
        Ok(())
    }
}

use std::collections::VecDeque;

use holochain_lib3h_protocol::{
    network_engine::{DidWork, NetworkEngine},
    protocol::Lib3hProtocol,
    Lib3hResult,
};

/// Struct holding all config settings for the RealEngine
#[derive(Debug, Clone, PartialEq)]
pub struct RealEngineConfig {
    pub socket_type: String,
    pub bootstrap_nodes: Vec<String>,
    pub work_dir: String,
    pub log_level: char,
}

/// Lib3h's 'real mode' as a NetworkEngine
pub struct RealEngine {
    /// Config settings
    config: RealEngineConfig,
    /// FIFO of messages received from Core
    inbox: VecDeque<Lib3hProtocol>,
    /// Identifier
    name: String,
}

impl RealEngine {
    /// Constructor
    pub fn new(config: RealEngineConfig, name: &str) -> Lib3hResult<Self> {
        Ok(RealEngine {
            config,
            inbox: VecDeque::new(),
            name: name.to_string(),
        })
    }

    /// Process a message sent by Core
    fn serve(&self, local_msg: Lib3hProtocol) -> Lib3hResult<(DidWork, Vec<Lib3hProtocol>)> {
        println!("(log.d) >>>> '{}' recv: {:?}", self.name.clone(), local_msg);
        let mut outbox = Vec::new();
        let mut did_work = false;
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
        Ok((did_work, outbox))
    }
}

impl NetworkEngine for RealEngine {
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

    /// Add incoming message in FIFO
    fn post(&mut self, local_msg: Lib3hProtocol) -> Lib3hResult<()> {
        self.inbox.push_back(local_msg);
        Ok(())
    }

    /// Process FIFO and output a list of protocol messages for Core to handle
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<Lib3hProtocol>)> {
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Progressively serve every protocol message in inbox
        loop {
            let local_msg = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            let (success, mut output) = self.serve(local_msg)?;
            if success {
                did_work = success;
            }
            outbox.append(&mut output);
        }
        Ok((did_work, outbox))
    }
}

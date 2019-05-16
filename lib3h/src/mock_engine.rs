
/// Lib3h's 'mock mode' as a NetworkEngine
struct MockEngine {
    /// End-user's network config
    config: serde_json::Value,
    /// Sender to the Worker
    tx: mpsc::Sender<Protocol>,
    /// identifier
    name: String,
}

impl MockEngine {
    pub fn new(config: serde_json::Value, tx: mpsc::Sender<Protocol>) -> NetResult<Self> {
        Ok(Lib3hMain {
            config,
            tx,
            name: "FIXME",
        })
    }
    /// Received message from p2p network.
    /// -> Process it or forward to local client
    fn receive(&mut self, data: Protocol) -> NetResult<()> {
        self.log
            .d(&format!("<<<< '{}' p2p recv: {:?}", self.name.clone(), data));
        // serve only JsonProtocol
        let maybe_json_msg = JsonProtocol::try_from(&data);
        if maybe_json_msg.is_err() {
            return Ok(());
        };
        // Note: use same order as the enum
        match maybe_json_msg.as_ref().unwrap() {
            JsonProtocol::SendMessageResult(msg) => {
                // FIXME
                self.tx.send(data)?;
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
        "FIXME"
    }

    /// process a message sent by our local Client
    fn serve(&mut self, data: Protocol) -> NetResult<()> {
        self.log
            .d(&format!(">>>> '{}' recv: {:?}", self.name.clone(), data));
        // serve only JsonProtocol
        let maybe_json_msg = JsonProtocol::try_from(&data);
        if maybe_json_msg.is_err() {
            return Ok(());
        };
        // Note: use same order as the enum
        match maybe_json_msg.as_ref().unwrap() {
            JsonProtocol::SuccessResult(msg) => {
                // FIXME
            }
            JsonProtocol::FailureResult(msg) => {
                // FIXME
            }
            JsonProtocol::TrackDna(msg) => {
                // FIXME
            }
            JsonProtocol::UntrackDna(msg) => {
                // FIXME
            }
            JsonProtocol::SendMessage(msg) => {
                // FIXME
            }
            JsonProtocol::HandleSendMessageResult(msg) => {
                // FIXME
            }
            JsonProtocol::FetchEntry(msg) => {
                // FIXME
            }
            JsonProtocol::HandleFetchEntryResult(msg) => {
                // FIXME
            }
            JsonProtocol::PublishEntry(msg) => {
                // FIXME
            }
            JsonProtocol::FetchMeta(msg) => {
                // FIXME
            }
            JsonProtocol::HandleFetchMetaResult(msg) => {
                // FIXME
            }
            JsonProtocol::PublishMeta(msg) => {
                // FIXME
            }
            // Our request for the publish_list has returned
            JsonProtocol::HandleGetPublishingEntryListResult(msg) => {
                // FIXME
            }
            // Our request for the hold_list has returned
            JsonProtocol::HandleGetHoldingEntryListResult(msg) => {
                // FIXME
            }
            // Our request for the publish_meta_list has returned
            JsonProtocol::HandleGetPublishingMetaListResult(msg) => {
                // FIXME
            }
            // Our request for the hold_meta_list has returned
            JsonProtocol::HandleGetHoldingMetaListResult(msg) => {
                // FIXME
            }
            _ => {
                panic!("unexpected {:?}", &maybe_json_msg);
            }
        }
        Ok(())
    }
}

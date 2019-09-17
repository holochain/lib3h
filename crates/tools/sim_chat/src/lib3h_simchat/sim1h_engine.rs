use lib3h_tracing::test_span;
use detach::{detach_run, Detach};
use lib3h::{engine::CanAdvertise, error::Lib3hError};


use lib3h_protocol::protocol::{
    ClientToLib3h, ClientToLib3hResponse, Lib3hToClient, Lib3hToClientResponse,
};
use lib3h_protocol::protocol_client::Lib3hClientProtocol;
use lib3h_protocol::protocol_server::Lib3hServerProtocol;
use lib3h_zombie_actor::{
    GhostActor, GhostCanTrack, GhostContextEndpoint, GhostEndpoint, GhostResult, WorkWasDone, GhostCallbackData::Response,
};
use url::Url;


/// This is an engine to interface with Sim1h
/// Sim1h communicates using the old client/server protocol so this
/// is mostly responsible for converting between those and redirecting
/// messages from the channel
pub struct Sim1hEngine<'engine> {
    client_endpoint: Option<
        GhostEndpoint<
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hToClient,
            Lib3hToClientResponse,
            Lib3hError,
        >,
    >,
    lib3h_endpoint: Detach<
        GhostContextEndpoint<
            Sim1hEngine<'engine>,
            Lib3hToClient,
            Lib3hToClientResponse,
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hError,
        >,
    >,
    //  This should own a lib1h instance and a channels
    sim1h_sender: crossbeam_channel::Sender<Lib3hClientProtocol>,
    sim1h_receiver: crossbeam_channel::Receiver<Lib3hServerProtocol>,
}

impl
    GhostActor<
        Lib3hToClient,
        Lib3hToClientResponse,
        ClientToLib3h,
        ClientToLib3hResponse,
        Lib3hError,
    > for Sim1hEngine<'_>
{
    // START BOILER PLATE--------------------------
    fn take_parent_endpoint(
        &mut self,
    ) -> Option<
        GhostEndpoint<
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hToClient,
            Lib3hToClientResponse,
            Lib3hError,
        >,
    > {
        std::mem::replace(&mut self.client_endpoint, None)
    }
    // END BOILER PLATE--------------------------

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // START BOILER PLATE--------------------------
        // always run the endpoint process loop
        detach_run!(&mut self.lib3h_endpoint, |cs| { cs.process(self) })?;
        // END BOILER PLATE--------------------------

        // grab any messages FROM THE SIM1H INSTANCE
        // convert these to the lib3h protocol and send over the client endpoint
        for sim1h_message in self.sim1h_receiver.try_iter() {
            
            // some will be responses to particular requests and should be handled as such

            let message_from_sim1h = Lib3hToClient::from(sim1h_message);

    		self.lib3h_endpoint.take().request(test_span(""), message_from_sim1h, Box::new(|_, callback_data| {
                // results get sent back over the channel
                if let Response(Ok(callback_message)) = callback_data {
                    let result_message = Lib3hClientProtocol::from(callback_message);
                    self.sim1h_sender.send(result_message)?;
                }
    			Ok(())
    		})).ok();
        }

    	// convert messages FROM CLIENT into protocol messages and pass down the channel
        for mut msg in self.lib3h_endpoint.as_mut().drain_messages() {
            let msg_to_sim1h = Lib3hClientProtocol::from(msg.take_message().expect("exists"));
        	self.sim1h_sender.send(msg_to_sim1h)?;
        }

        Ok(true.into())
    }
}

impl CanAdvertise for Sim1hEngine<'_> {
    fn advertise(&self) -> Url {
        Url::parse("ws://mock_peer_url").unwrap()
    }
}

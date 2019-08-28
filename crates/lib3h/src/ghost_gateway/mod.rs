pub mod gateway_actor;
pub mod ghost_gateway;
pub mod gateway_dht;

use crate::{
    dht::dht_trait::Dht,
    transport::protocol::*,
};
use lib3h_ghost_actor::prelude::*;
use detach::prelude::*;

/// Gateway Actor where:
///   - child: some Transport
///   - parent: RealEngine or Multiplexer
/// Transport protocol used on all ends
pub struct GhostGateway<D: Dht> {
    /// Used for distinguishing gateways
    identifier: String,
    /// Internal DHT
    inner_dht: D,
    /// Hold child transport actor
    child_transport: Detach<TransportParentWrapper>,
    /// Channel to our parent actor
    endpoint_parent: Option<TransportEndpoint>,
    endpoint_self: Option<TransportEndpointWithContext>,
}


#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;
    use crate::{
        dht::mirror_dht::MirrorDht,
        dht::dht_trait::{Dht, DhtConfig, DhtFactory},
        engine::NETWORK_GATEWAY_ID,
    };

    fn create_ghost_gateway(peer_address: &str) -> GhostGateway<MirrorDht> {
        // Create memory Transport and bind
        let mut memory_transport = GhostTransportMemory::new();
        let mut parent_endpoint = memory_transport
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint::<()>("tmem_to_child1");
        let mut binding = Url::parse("").unwrap();
        parent_endpoint.request(
            std::time::Duration::from_millis(200),
            (),
            TransportRequestToChild::Bind {
                spec: Url::parse("mem://_").unwrap(),
            },
            Box::new(|_, _, r| {
                binding = r;
                Ok(())
            }),);
        memory_transport.process();
        let _ = parent_endpoint.process(&mut ());
        // Wait for bind response
        let mut timeout = 0;
        while binding == Url::parse("").unwrap() && timeout < 200 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            timeout += 10;
        }
        assert!(timeout < 200);
        // Generate DHT config and create gateway
        let dht_config = DhtConfig::new(peer_address, &binding);
        GhostGateway::new(
            NETWORK_GATEWAY_ID,
            memory_transport.clone(),
            MirrorDht::new_with_config,
            &dht_config,
        )
    }
    #[test]
    fn test_gateway_as_transport() {

        let mut gateway_a = create_ghost_gateway("peer_a");
        let mut gateway_b = create_ghost_gateway("peer_b");

        let expected_address_a = Url::parse("mem://addr_1").unwrap();
        let expected_address_b = Url::parse("mem://addr_2").unwrap();

//        assert_eq!(
//            gateway_a.maybe_my_address,
//            Some(expected_transport1_address)
//        );
//        assert_eq!(
//            gateway_b.maybe_my_address,
//            Some(expected_transport2_address)
//        );

        let mut endpoint_a = gateway_a
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint::<()>("gateway_a_endpoint");
        let mut endpoint_b = gateway_b
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint::<()>("gateway_b_endpoint");
        // now send a message from transport1 to transport2 over the bound addresses
        endpoint_a.request(
            std::time::Duration::from_millis(2000),
            (),
            TransportRequestToChild::SendMessage {
                address: Url::parse("mem://addr_2").unwrap(),
                payload: b"test message".to_vec(),
            },
            Box::new(|_, _, r| {
                // parent should see that the send request was OK
                assert_eq!("Response(Ok(SendMessage))", &format!("{:?}", r));
                Ok(())
            }),
        );

        gateway_a.process().unwrap();
        let _ = endpoint_a.process(&mut ());

        gateway_b.process().unwrap();
        let _ = endpoint_b.process(&mut ());

        let mut requests = endpoint_b.drain_messages();
        assert_eq!(2, requests.len());
        assert_eq!(
            "Some(IncomingConnection { address: \"mem://addr_1/\" })",
            format!("{:?}", requests[0].take_message())
        );
        assert_eq!("Some(ReceivedData { address: \"mem://addr_1/\", payload: [116, 101, 115, 116, 32, 109, 101, 115, 115, 97, 103, 101] })",format!("{:?}",requests[1].take_message()));
    }
}


//
//#[cfg(test)]
//mod tests {
//    use super::*;
//    use detach::prelude::*;
//    use std::any::Any;
//
//
//    type FakeError = String;
//
//    #[test]
//    fn test_ghost_gateway() {
//
//    }
//
//    #[test]
//    fn test_ghost_gateway_example() {
//        // the body of this test simulates an object that contains a actor, i.e. a parent.
//        // it would usually just be another ghost_actor but here we test it out explicitly
//        // so first instantiate the "child" actor
//        let mut t_actor: TransportActor = Box::new(GatewayTransport::new());
//        let mut t_actor_endpoint = t_actor
//            .take_parent_endpoint()
//            .expect("exists")
//            .as_context_endpoint::<i8>("test");
//
//        // allow the actor to run this actor always creates a simulated incoming
//        // connection each time it processes
//        t_actor.process().unwrap();
//        let _ = t_actor_endpoint.process(&mut ());
//
//        // now process any requests the actor may have made of us (as parent)
//        for mut msg in t_actor_endpoint.drain_messages() {
//            let payload = msg.take_message();
//            println!("in drain_messages got: {:?}", payload);
//
//            // we might allow or disallow connections for example
//            let response = RequestToParentResponse::Allowed;
//            msg.respond(Ok(response));
//        }
//
//        t_actor.process().unwrap();
//        let _ = t_actor_endpoint.process(&mut ());
//
//        // now make a request of the child,
//        // to make such a request the parent would normally will also instantiate trackers so that it can
//        // handle responses when they come back as callbacks.
//        // here we simply watch that we got a response back as expected
//        t_actor_endpoint.request(
//            std::time::Duration::from_millis(2000),
//            42_i8,
//            RequestToChild::Bind {
//                spec: "address_to_bind_to".to_string(),
//            },
//            Box::new(|_, _, r| {
//                println!("in callback 1, got: {:?}", r);
//                Ok(())
//            }),
//        );
//
//        t_actor.process().unwrap();
//        let _ = t_actor_endpoint.process(&mut ());
//
//        t_actor_endpoint.request(
//            std::time::Duration::from_millis(2000),
//            42_i8,
//            RequestToChild::SendMessage {
//                address: "agent_id_1".to_string(),
//                payload: b"some content".to_vec(),
//            },
//            Box::new(|_, _, r| {
//                println!("in callback 2, got: {:?}", r);
//                Ok(())
//            }),
//        );
//
//        for _x in 0..10 {
//            t_actor.process().unwrap();
//            let _ = t_actor_endpoint.process(&mut ());
//        }
//    }
//}
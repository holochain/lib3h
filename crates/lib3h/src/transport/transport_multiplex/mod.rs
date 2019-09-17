//! Let's say we have two agentIds: a1 and a2
//! a1 is running on machineId: m1
//! a2 is running on machineId: m2
//!
//! The AgentSpaceGateway will wrap messages in a p2p_proto direct message:
//!   DirectMessage {
//!     space_address: "Qmyada",
//!     to_agent_id: "a2",
//!     from_agent_id: "a1",
//!     payload: <...>,
//!   }
//!
//! Then send it to the machine id:
//!   dest: "m2", payload: <above, but binary>
//!
//! When the multiplexer receives data (at the network/machine gateway),
//! if it is any other p2p_proto message, it will be forwarded to
//! the engine or network gateway. If it is a direct message, it will be
//! sent to the appropriate Route / AgentSpaceGateway

mod mplex;
pub use mplex::TransportMultiplex;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{error::Lib3hError, gateway::protocol::*, transport::protocol::*};
    use detach::prelude::*;
    use lib3h_ghost_actor::prelude::*;
    use lib3h_protocol::data_types::Opaque;
    use lib3h_tracing::Lib3hSpan;
    use url::Url;

    pub struct GatewayMock {
        endpoint_parent: Option<GatewayParentEndpoint>,
        endpoint_self: Detach<
            GhostContextEndpoint<
                GatewayMock,
                GatewayRequestToParent,
                GatewayRequestToParentResponse,
                GatewayRequestToChild,
                GatewayRequestToChildResponse,
                Lib3hError,
            >,
        >,
        bound_url: Url,
        mock_sender: crossbeam_channel::Sender<(Url, Opaque)>,
        mock_receiver: crossbeam_channel::Receiver<(Url, Opaque)>,
    }

    impl GatewayMock {
        pub fn new(
            mock_sender: crossbeam_channel::Sender<(Url, Opaque)>,
            mock_receiver: crossbeam_channel::Receiver<(Url, Opaque)>,
        ) -> Self {
            let (endpoint_parent, endpoint_self) = create_ghost_channel();
            let endpoint_parent = Some(endpoint_parent);
            let endpoint_self = Detach::new(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix("mock_to_parent_")
                    .build(),
            );
            Self {
                endpoint_parent,
                endpoint_self,
                bound_url: Url::parse("none:").expect("can parse url"),
                mock_sender,
                mock_receiver,
            }
        }
    }

    impl
        GhostActor<
            GatewayRequestToParent,
            GatewayRequestToParentResponse,
            GatewayRequestToChild,
            GatewayRequestToChildResponse,
            Lib3hError,
        > for GatewayMock
    {
        fn take_parent_endpoint(&mut self) -> Option<GatewayParentEndpoint> {
            std::mem::replace(&mut self.endpoint_parent, None)
        }

        fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
            detach_run!(&mut self.endpoint_self, |es| es.process(self))?;
            for mut msg in self.endpoint_self.as_mut().drain_messages() {
                match msg.take_message().expect("exists") {
                    GatewayRequestToChild::Transport(req) => match req {
                        RequestToChild::Bind { mut spec } => {
                            spec.set_path("bound");
                            self.bound_url = spec.clone();
                            msg.respond(Ok(GatewayRequestToChildResponse::Transport(
                                RequestToChildResponse::Bind(BindResultData { bound_url: spec }),
                            )))?;
                        }
                        RequestToChild::SendMessage { uri, payload } => {
                            self.mock_sender.send((uri, payload))?;
                            msg.respond(Ok(GatewayRequestToChildResponse::Transport(
                                RequestToChildResponse::SendMessageSuccess,
                            )))?;
                        }
                    },
                    _ => unimplemented!(),
                }
            }
            loop {
                let span = Lib3hSpan::fixme();
                match self.mock_receiver.try_recv() {
                    Ok((uri, payload)) => {
                        self.endpoint_self.publish(
                            span,
                            GatewayRequestToParent::Transport(RequestToParent::ReceivedData {
                                uri,
                                payload,
                            }),
                        )?;
                    }
                    Err(_) => break,
                }
            }
            Ok(false.into())
        }
    }

    #[test]
    fn it_should_multiplex() {
        let (s_out, r_out) = crossbeam_channel::unbounded();
        let (s_in, r_in) = crossbeam_channel::unbounded();

        let addr_none = Url::parse("none:").expect("can parse url");

        let mut mplex: GatewayParentWrapper<(), TransportMultiplex<GatewayMock>> =
            GhostParentWrapper::new(
                TransportMultiplex::new(GatewayMock::new(s_out, r_in)),
                "test_mplex_",
            );

        let mut route_a = mplex
            .as_mut()
            .create_agent_space_route(&"space_a".into(), &"agent_a".into())
            .as_context_endpoint_builder()
            .build::<()>();

        let mut route_b = mplex
            .as_mut()
            .create_agent_space_route(&"space_b".into(), &"agent_b".into())
            .as_context_endpoint_builder()
            .build::<()>();

        // send a message from route A
        route_a
            .request(
                Lib3hSpan::fixme(),
                RequestToChild::SendMessage {
                    uri: addr_none.clone(),
                    payload: "hello-from-a".into(),
                },
                Box::new(|_, response| {
                    assert_eq!(&format!("{:?}", response), "");
                    Ok(())
                }),
            )
            .unwrap();

        route_a.process(&mut ()).unwrap();
        mplex.process(&mut ()).unwrap();
        route_a.process(&mut ()).unwrap();
        mplex.process(&mut ()).unwrap();

        // should receive that out the bottom
        let (address, payload) = r_out.recv().unwrap();
        assert_eq!(&addr_none, &address);
        let expected: Opaque = "hello-from-a".into();
        assert_eq!(&expected, &payload);

        // send a message up the bottom
        s_in.send((addr_none.clone(), "hello-to-b".into())).unwrap();

        // process "receive" that message
        mplex.process(&mut ()).unwrap();
        let mut msgs = mplex.drain_messages();
        assert_eq!(1, msgs.len());

        let msg = msgs.remove(0).take_message().unwrap();
        if let GatewayRequestToParent::Transport(RequestToParent::ReceivedData { uri, payload }) =
            msg
        {
            assert_eq!(&addr_none, &uri);
            let expected: Opaque = "hello-to-b".into();
            assert_eq!(&expected, &payload);
        } else {
            panic!("bad type");
        }

        // our mplex module got it, now we should have the context
        // let's instruct it to be forwarded up the route
        mplex
            .as_mut()
            .received_data_for_agent_space_route(
                &"space_b".into(),
                &"agent_b".into(),
                &"agent_x".into(),
                &"machine_x".into(),
                "hello".into(),
            )
            .unwrap();

        mplex.process(&mut ()).unwrap();
        route_b.process(&mut ()).unwrap();

        let mut msgs = route_b.drain_messages();
        assert_eq!(1, msgs.len());

        let msg = msgs.remove(0).take_message().unwrap();
        if let RequestToParent::ReceivedData { uri, payload } = msg {
            assert_eq!(
                &Url::parse("transportid:machine_x?a=agent_x").unwrap(),
                &uri
            );
            let expected: Opaque = "hello".into();
            assert_eq!(&expected, &payload);
        } else {
            panic!("bad type");
        }
    }
}

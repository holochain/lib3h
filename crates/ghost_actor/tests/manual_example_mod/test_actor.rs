//! would love to do some code generation for the actual actor
//! but haven't figured out the specific path yet. Derive? Attribute?
//! for now, all this code is hand-written

use super::test_protocol::*;
use ghost_actor::prelude::*;

pub struct TestActor<'lt> {
    #[allow(dead_code)]
    sys_ref: GhostActorSystem<'lt, Self>,
    owner_ref: GhostEndpointFull<'lt, TestProtocol, (), Self, TestActorHandler<'lt, Self>>,
    sub_actor:
        Option<GhostEndpointFull<'lt, TestProtocol, Self, Self, TestActorHandler<'lt, Self>>>,
}

impl<'lt> TestActor<'lt> {
    pub fn new(
        mut sys_ref: GhostActorSystem<'lt, Self>,
        owner_seed: GhostEndpointSeed<'lt, TestProtocol, ()>,
        sub_actor: Option<GhostEndpointSeed<'lt, TestProtocol, Self>>,
    ) -> GhostResult<Self> {
        let sub_actor = match sub_actor {
            None => None,
            _ => unimplemented!(),
        };

        let owner_ref = sys_ref.plant_endpoint(
            owner_seed,
            TestActorHandler {
                handle_event_to_actor_print: Box::new(|me: &mut TestActor<'lt>, message| {
                    match &mut me.sub_actor {
                        None => {
                            me.owner_ref
                                .event_to_owner_print(format!("echo: {:?}", message))?;
                            println!("actor printing message from owner: {}", message);
                        }
                        Some(sub_actor) => {
                            sub_actor.event_to_actor_print(message)?;
                        }
                    }
                    Ok(())
                }),
                handle_request_to_actor_add_1: Box::new(|me: &mut TestActor<'lt>, message, cb| {
                    match &mut me.sub_actor {
                        None => cb(Ok(message + 1)),
                        Some(sub_actor) => sub_actor.request_to_actor_add_1(
                            message,
                            Box::new(move |_me, result| cb(result?)),
                        ),
                    }
                }),
            },
        )?;

        let mut out = Self {
            sys_ref,
            owner_ref,
            sub_actor,
        };
        out.owner_ref
            .event_to_owner_print("message from actor".to_string())?;
        out.owner_ref.request_to_owner_sub_1(
            42,
            Box::new(|me, result| {
                me.owner_ref
                    .event_to_owner_print(format!("echo: {:?}", result))?;
                println!("actor got response from owner: 42 - 1 = {:?}", result);
                Ok(())
            }),
        )?;
        Ok(out)
    }

    pub fn test_mut_method(&mut self) -> String {
        "test_mut_method_data".to_string()
    }
}

impl<'lt> GhostActor<'lt, TestProtocol, TestActor<'lt>> for TestActor<'lt> {
    fn process(&mut self) -> GhostResult<()> {
        println!("process called");
        Ok(())
    }
}

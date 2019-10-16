//! would love to do some code generation for the actual actor
//! but haven't figured out the specific path yet. Derive? Attribute?
//! for now, all this code is hand-written

use super::test_protocol::*;
use ghost_actor::prelude::*;

pub struct TestActor<'lt> {
    name: String,
    #[allow(dead_code)]
    sys_ref: GhostActorSystem<'lt, Self>,
    owner_ref: GhostEndpointFull<'lt, TestProtocol, (), Self, TestActorHandler<'lt, Self>>,
    sub_actor:
        Option<GhostEndpointFull<'lt, TestProtocol, Self, Self, TestOwnerHandler<'lt, Self>>>,
}

impl<'lt> TestActor<'lt> {
    pub fn new(
        name: &str,
        mut sys_ref: GhostActorSystem<'lt, Self>,
        owner_seed: GhostEndpointSeed<'lt, TestProtocol, ()>,
        sub_actor: Option<GhostEndpointSeed<'lt, TestProtocol, Self>>,
    ) -> GhostResult<Self> {
        let sub_actor = if name == "root" {
            if let Some(_) = sub_actor {
                panic!("expected None if at root")
            }
            // if we are at the root, construct a sub_1 actor with a sub_2 actor
            let sub_2 = Some(sys_ref.spawn_seed(Box::new(|sys_ref, owner_seed| {
                TestActor::new("sub_2", sys_ref, owner_seed, None)
            }))?);

            Some(sys_ref.spawn_seed(Box::new(move |sys_ref, owner_seed| {
                TestActor::new("sub_1", sys_ref, owner_seed, sub_2)
            }))?)
        } else {
            sub_actor
        };

        let sub_actor = match sub_actor {
            Some(sub_actor) => Some(sys_ref.plant_endpoint(
                sub_actor,
                TestOwnerHandler {
                    handle_event_to_owner_print: Box::new(|me: &mut Self, message| {
                        me.owner_ref
                            .event_to_owner_print(format!("({} chain {})", me.name, message))
                    }),
                    handle_request_to_owner_sub_1: Box::new(|me, message, cb| {
                        me.owner_ref.request_to_owner_sub_1(
                            message,
                            Box::new(move |me, resp| {
                                me.owner_ref.event_to_owner_print(format!(
                                    "({} fwd sub_1 {:?}",
                                    me.name, resp
                                ))?;
                                cb(resp?)
                            }),
                        )
                    }),
                },
            )?),
            None => None,
        };

        let mut owner_ref = sys_ref.plant_endpoint(
            owner_seed,
            TestActorHandler {
                handle_event_to_actor_print: Box::new(|me: &mut TestActor<'lt>, message| {
                    match &mut me.sub_actor {
                        None => {
                            me.owner_ref.event_to_owner_print(format!(
                                "({} recv print {})",
                                me.name, message
                            ))?;
                        }
                        Some(sub_actor) => {
                            sub_actor.event_to_actor_print(format!(
                                "({} fwd print {})",
                                me.name, message
                            ))?;
                        }
                    }
                    Ok(())
                }),
                handle_request_to_actor_add_1: Box::new(|me: &mut TestActor<'lt>, message, cb| {
                    match &mut me.sub_actor {
                        None => {
                            me.owner_ref.event_to_owner_print(format!(
                                "({} add 1 to {})",
                                me.name, message
                            ))?;
                            cb(Ok(message + 1))
                        }
                        Some(sub_actor) => sub_actor.request_to_actor_add_1(
                            message,
                            Box::new(move |me, result| {
                                me.owner_ref.event_to_owner_print(format!(
                                    "({} fwd add_1 request)",
                                    me.name
                                ))?;
                                cb(result?)
                            }),
                        ),
                    }
                }),
            },
        )?;

        if sub_actor.is_none() {
            // we are the lowest level, send up some events/requests
            owner_ref.event_to_owner_print(format!("({} to_owner_print)", name))?;
            let name_clone = name.to_string();
            owner_ref.request_to_owner_sub_1(
                42,
                Box::new(move |me, result| {
                    me.owner_ref.event_to_owner_print(format!(
                        "({} rsp 42 - 1 = {:?})",
                        name_clone, result
                    ))?;
                    Ok(())
                }),
            )?;
        }

        let out = Self {
            name: name.to_string(),
            sys_ref,
            owner_ref,
            sub_actor,
        };
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

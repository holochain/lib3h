//! would love to do some code generation for the actual actor
//! but haven't figured out the specific path yet. Derive? Attribute?
//! for now, all this code is hand-written

use super::test_protocol::*;
use ghost_actor::prelude::*;
use holochain_tracing::*;

pub struct TestActor<'lt, S: GhostSystemRef<'lt>> {
    name: String,
    #[allow(dead_code)]
    sys_ref: GhostActorSystem<'lt, Self, S>,
    owner_ref: GhostEndpointFull<'lt, TestProtocol, (), Self, TestActorHandler<'lt, Self>, S>,
    maybe_sub_actor:
        Option<GhostEndpointFull<'lt, TestProtocol, Self, Self, TestOwnerHandler<'lt, Self>, S>>,
}

impl<'lt, S: GhostSystemRef<'lt>> TestActor<'lt, S> {
    /// Getter
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Create a TestActor from seeds
    pub fn new(
        name: &str,
        mut sys_ref: GhostActorSystem<'lt, Self, S>,
        owner_seed: GhostEndpointSeed<'lt, TestProtocol, (), S>,
        maybe_sub_actor_seed: Option<GhostEndpointSeed<'lt, TestProtocol, Self, S>>,
    ) -> GhostResult<Self> {
        // if we are the root, construct a sub_1 actor seed with a sub_2 actor seed
        let maybe_sub_actor_seed = if name == "root" {
            if let Some(_) = maybe_sub_actor_seed {
                panic!("expected None sub_actor_seed when root")
            }
            let sub_2 = sys_ref.spawn_seed(Box::new(|sys_ref, owner_seed| {
                TestActor::new("sub_2", sys_ref, owner_seed, None)
            }))?;

            let sub_1 = sys_ref.spawn_seed(Box::new(move |sys_ref, owner_seed| {
                TestActor::new("sub_1", sys_ref, owner_seed, Some(sub_2))
            }))?;
            Some(sub_1)
        } else {
            maybe_sub_actor_seed
        };

        // Plant sub actor seed
        let maybe_sub_actor = match maybe_sub_actor_seed {
            Some(sub_actor_seed) => {
                let sub_actor = sys_ref.plant_seed(
                    sub_actor_seed,
                    TestOwnerHandler {
                        handle_event_to_owner_print: Box::new(|span, me: &mut Self, message| {
                            me.owner_ref.event_to_owner_print(
                                Some(span),
                                format!("({} chain {})", me.name, message),
                            )
                        }),
                        handle_request_to_owner_sub_1: Box::new(|span, me, message, cb| {
                            me.owner_ref.request_to_owner_sub_1(
                                Some(span),
                                message,
                                Box::new(move |span, me, resp| {
                                    me.owner_ref.event_to_owner_print(
                                        Some(span.child("sub_1 request_to_owner_sub_1")),
                                        format!("({} fwd sub_1 {:?}", me.name, resp),
                                    )?;
                                    cb(span, resp?)
                                }),
                            )
                        }),
                    },
                )?;
                Some(sub_actor)
            }
            None => None,
        };

        // Plant owner seed
        let mut owner_ref = sys_ref.plant_seed(
            owner_seed,
            TestActorHandler {
                handle_event_to_actor_print: Box::new(
                    |span: Span, me: &mut TestActor<'lt, S>, message| {
                        match &mut me.maybe_sub_actor {
                            None => {
                                me.owner_ref.event_to_owner_print(
                                    Some(span),
                                    format!("({} recv print {})", me.name, message),
                                )?;
                            }
                            Some(sub_actor) => {
                                sub_actor.event_to_actor_print(
                                    Some(span),
                                    format!("({} fwd print {})", me.name, message),
                                )?;
                            }
                        }
                        Ok(())
                    },
                ),
                handle_request_to_actor_add_1: Box::new(
                    |span: Span, me: &mut TestActor<'lt, S>, message, cb| match &mut me
                        .maybe_sub_actor
                    {
                        None => {
                            me.owner_ref.event_to_owner_print(
                                Some(span),
                                format!("({} add 1 to {})", me.name, message),
                            )?;
                            cb(Span::fixme(), Ok(message + 1))
                        }
                        Some(sub_actor) => sub_actor.request_to_actor_add_1(
                            Some(span),
                            message,
                            Box::new(move |span, me, result| {
                                me.owner_ref.event_to_owner_print(
                                    Some(span.child("add_1 request")),
                                    format!("({} fwd add_1 request)", me.name),
                                )?;
                                cb(span, result?)
                            }),
                        ),
                    },
                ),
            },
        )?;

        // if we are the lowest level, send some events/requests to owner
        if maybe_sub_actor.is_none() {
            let name_clone = name.to_string();
            let tracer = TRACER_SINGLETON.lock().unwrap();
            let mut span: Span = tracer.span("TestActor leaf creation").start().into();
            span.set_tag(|| Tag::new("actor", name_clone.clone()));
            owner_ref.event_to_owner_print(
                Some(span.child("event_to_owner_print")),
                format!("({} to_owner_print)", name),
            )?;
            owner_ref.request_to_owner_sub_1(
                Some(span.child("request_to_owner_sub_1")),
                42,
                Box::new(move |span, me, result| {
                    me.owner_ref.event_to_owner_print(
                        Some(span),
                        format!("({} rsp 42 - 1 = {:?})", name_clone, result),
                    )?;
                    Ok(())
                }),
            )?;
        }

        // Done
        let out = Self {
            name: name.to_string(),
            sys_ref,
            owner_ref,
            maybe_sub_actor,
        };
        Ok(out)
    }

    /// Dummy method for testing out a mut self method
    pub fn test_mut_method(&mut self) -> String {
        "test_mut_method_data".to_string()
    }
}

impl<'lt, S: GhostSystemRef<'lt> + Send + Sync> GhostActor<'lt, TestProtocol, TestActor<'lt, S>>
    for TestActor<'lt, S>
{
    fn process(&mut self) -> GhostResult<()> {
        println!("TestActor.process called");
        Ok(())
    }
}

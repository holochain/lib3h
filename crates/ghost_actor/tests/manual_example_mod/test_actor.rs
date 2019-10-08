//! would love to do some code generation for the actual actor
//! but haven't figured out the specific path yet. Derive? Attribute?
//! for now, all this code is hand-written

use super::test_protocol::*;
use ghost_actor::prelude::*;

pub struct TestActor<'lt> {
    owner_ref: GhostEndpointRef<'lt, Self, (), TestProtocol, TestActorHandler<'lt, Self>>,
}

impl<'lt> TestActor<'lt> {
    pub fn new(inflator: GhostInflator<'lt, TestProtocol, Self>) -> GhostResult<Self> {
        let mut out = Self {
            owner_ref: inflator.inflate(TestActorHandler {
                handle_event_to_actor_print: Box::new(|me: &mut TestActor<'lt>, message| {
                    me.owner_ref
                        .event_to_owner_print(format!("echo: {:?}", message))?;
                    println!("actor printing message from owner: {}", message);
                    Ok(())
                }),
                handle_request_to_actor_add_1: Box::new(|_me: &mut TestActor<'lt>, message, cb| {
                    cb(Ok(message + 1))
                }),
            })?,
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
}

impl<'lt> GhostActor<'lt, TestProtocol, TestActor<'lt>> for TestActor<'lt> {
    fn process(&mut self) -> GhostResult<()> {
        println!("process called");
        Ok(())
    }
}

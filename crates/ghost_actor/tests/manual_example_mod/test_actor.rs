//! would love to do some code generation for the actual actor
//! but haven't figured out the specific path yet. Derive? Attribute?
//! for now, all this code is hand-written

use super::test_protocol::*;
use ghost_actor::prelude::*;

pub struct TestActor<'lt> {
    system_ref: Option<GhostSystemRef<'lt>>,
    owner_ref: Option<GhostEndpointRef<'lt, Self, (), TestProtocol, TestActorHandler<'lt, Self>>>,
}

impl<'lt> TestActor<'lt> {
    pub fn new() -> Self {
        Self {
            system_ref: None,
            owner_ref: None,
        }
    }
}

impl<'lt> GhostActor<'lt, TestProtocol, TestActor<'lt>> for TestActor<'lt> {
    fn actor_init<'a>(
        &'a mut self,
        inflator: GhostInflator<'a, 'lt, TestActor<'lt>, TestProtocol>,
    ) -> GhostResult<()> {
        let (system_ref, mut owner_ref) = inflator.inflate(TestActorHandler {
            phantom: std::marker::PhantomData,
            handle_event_to_actor_print: Box::new(|_me: &mut TestActor<'lt>, message| {
                println!("actor print: {}", message);
                Ok(())
            }),
            handle_request_to_actor_add_1: Box::new(|_me: &mut TestActor<'lt>, message, cb| {
                cb(Ok(message + 1))
            }),
        })?;
        owner_ref.event_to_owner_print("message from actor".to_string())?;
        owner_ref.request_to_owner_sub_1(
            42,
            Box::new(|_me, result| {
                println!("got sub from owner: 42 - 1 = {:?}", result);
                Ok(())
            }),
        )?;

        self.system_ref = Some(system_ref);
        self.owner_ref = Some(owner_ref);

        Ok(())
    }

    fn process(&mut self) -> GhostResult<()> {
        println!("process called");
        Ok(())
    }
}

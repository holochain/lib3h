use std::sync::{Arc, Mutex};

use ghost_actor::prelude::*;

mod manual_example_mod;
#[allow(unused_imports)]
use manual_example_mod::*;

#[test]
fn manual_example() {
    let mut system = GhostSystem::new();
    let mut system_ref = system.create_ref();

    struct MyContext {}
    let my_context = Arc::new(Mutex::new(MyContext {}));
    let my_context_weak = Arc::downgrade(&my_context);

    let mut actor_ref = system_ref
        .spawn::<MyContext, TestProtocol, TestActor, TestOwnerHandler<MyContext>>(
            my_context_weak,
            TestActor::new(),
            TestOwnerHandler {
                handle_event_to_owner_print: Box::new(|_me, message| {
                    println!("owner printing message from actor: {}", message);
                    Ok(())
                }),
                handle_request_to_owner_sub_1: Box::new(|_me, message, cb| cb(Ok(message - 1))),
            },
        )
        .unwrap();

    actor_ref
        .event_to_actor_print("zombies".to_string())
        .unwrap();
    actor_ref
        .request_to_actor_add_1(
            42,
            Box::new(|_, rsp| {
                println!("owner got response from actor: 42 + 1 = {:?}", rsp);
                Ok(())
            }),
        )
        .unwrap();

    system.process().unwrap();
    system.process().unwrap();
}

use std::sync::{Arc, Mutex};

use ghost_actor::prelude::*;

mod manual_example_mod;
#[allow(unused_imports)]
use manual_example_mod::*;

#[test]
fn manual_example() {
    let mut system = GhostSystem::new();
    let system_ref = system.create_ref();

    #[derive(Debug)]
    struct MyContext {
        to_owner_prints: Vec<String>,
        to_actor_add_resp: Vec<String>,
    }

    let my_context = Arc::new(Mutex::new(MyContext {
        to_owner_prints: Vec::new(),
        to_actor_add_resp: Vec::new(),
    }));

    let my_context_weak = Arc::downgrade(&my_context);

    let mut actor_ref =
        ghost_actor_spawn::<MyContext, TestProtocol, TestActor, TestOwnerHandler<MyContext>>(
            system_ref.clone(),
            my_context_weak,
            Box::new(|inflator| TestActor::new(inflator)),
            TestOwnerHandler {
                handle_event_to_owner_print: Box::new(|me, message| {
                    me.to_owner_prints.push(message.clone());
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
            Box::new(|me, rsp| {
                me.to_actor_add_resp.push(format!("{:?}", rsp));
                println!("owner got response from actor: 42 + 1 = {:?}", rsp);
                Ok(())
            }),
        )
        .unwrap();

    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();

    assert_eq!("MyContext { to_owner_prints: [\"message from actor\", \"echo: \\\"zombies\\\"\", \"echo: Ok(Ok(41))\"], to_actor_add_resp: [\"Ok(Ok(43))\"] }", &format!("{:?}", my_context.lock().unwrap()));
    println!("{:#?}", my_context);
}

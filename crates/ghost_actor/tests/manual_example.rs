use std::sync::Arc;

use ghost_actor::prelude::*;

mod manual_example_mod;
#[allow(unused_imports)]
use manual_example_mod::*;

#[test]
fn manual_example() {
    let mut system = SingleThreadedGhostSystem::new();

    #[derive(Debug)]
    struct MyContext {
        to_owner_prints: Vec<String>,
        to_actor_add_resp: Vec<String>,
    }

    let my_context = Arc::new(GhostMutex::new(MyContext {
        to_owner_prints: Vec::new(),
        to_actor_add_resp: Vec::new(),
    }));

    let my_context_weak = Arc::downgrade(&my_context);

    let (mut system_ref, finalize) = system.create_external_system_ref();
    finalize(my_context_weak).unwrap();

    let mut actor_ref = system_ref
        .spawn(
            Box::new(|sys_ref, owner_seed| TestActor::new("root", sys_ref, owner_seed, None)),
            TestOwnerHandler {
                handle_event_to_owner_print: Box::new(|me: &mut MyContext, message| {
                    me.to_owner_prints.push(message);
                    Ok(())
                }),
                handle_request_to_owner_sub_1: Box::new(|_me: &mut MyContext, message, cb| {
                    cb(Ok(message - 1))
                }),
            },
        )
        .unwrap();

    actor_ref
        .event_to_actor_print("test-from-framework".to_string())
        .unwrap();
    actor_ref
        .request_to_actor_add_1(
            42,
            Box::new(|me, rsp| {
                me.to_actor_add_resp.push(format!("{:?}", rsp));
                Ok(())
            }),
        )
        .unwrap();

    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();
    system.process().unwrap();

    assert_eq!("MyContext { to_owner_prints: [\"(root chain (sub_1 chain (sub_2 to_owner_print)))\", \"(root fwd sub_1 Ok(Ok(41))\", \"(root chain (sub_1 fwd sub_1 Ok(Ok(41)))\", \"(root chain (sub_1 fwd add_1 request))\", \"(root chain (sub_1 chain (sub_2 recv print (sub_1 fwd print (root fwd print test-from-framework)))))\", \"(root chain (sub_1 chain (sub_2 add 1 to 42)))\", \"(root chain (sub_1 chain (sub_2 rsp 42 - 1 = Ok(Ok(41)))))\", \"(root fwd add_1 request)\"], to_actor_add_resp: [\"Ok(Ok(43))\"] }", &format!("{:?}", my_context.lock()));
    println!("{:#?}", my_context);

    // can we access it directly?
    assert_eq!(
        "test_mut_method_data",
        &actor_ref.as_mut().test_mut_method()
    );
}

use std::sync::Arc;

use ghost_actor::prelude::*;
use holochain_tracing::*;

mod manual_example_mod;
#[allow(unused_imports)]
use manual_example_mod::*;

#[test]
fn manual_example() {
    let mut actor_system = SingleThreadedGhostSystem::new();

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

    let (mut system_ref, finalize) = actor_system.create_external_system_ref();
    finalize(my_context_weak).unwrap();

    let mut actor_ref = system_ref
        .spawn(
            Box::new(|sys_ref, owner_seed| TestActor::new("root", sys_ref, owner_seed, None)),
            TestOwnerHandler {
                handle_event_to_owner_print: Box::new(|mut span, me: &mut MyContext, message| {
                    span.event("print");
                    me.to_owner_prints.push(message);
                    Ok(())
                }),
                handle_request_to_owner_sub_1: Box::new(
                    |span, _me: &mut MyContext, message, cb| {
                        cb(span.child("sub_1 response"), Ok(message - 1))
                    },
                ),
            },
        )
        .unwrap();

    // Creates a tracer
    let (span_tx, span_rx) = crossbeam_channel::bounded(10);
    let tracer = Tracer::with_sender(AllSampler, span_tx);
    // Starts "root" span
    {
        let root_span: HSpan = tracer.span("manual_example_span").start().into();
        //let root_span = test_span("manual_example_span");

        actor_ref
            .event_to_actor_print(
                Some(root_span.follower("first event")),
                "test-from-framework".to_string(),
            )
            .unwrap();

        actor_ref
            .request_to_actor_add_1(
                Some(root_span.follower("first request")),
                42,
                Box::new(|mut span, me, rsp| {
                    span.event(format!("{:?}", rsp));
                    me.to_actor_add_resp.push(format!("{:?}", rsp));
                    Ok(())
                }),
            )
            .unwrap();

        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
        actor_system.process().unwrap();
    }
    //println!("root_span: {:?}", root_span);
    // Outputs finished spans to the standard output
    while let Ok(span) = span_rx.try_recv() {
        println!("# SPAN: {:?}", span);
    }

    assert_eq!("MyContext { to_owner_prints: [\"(root chain (sub_1 chain (sub_2 to_owner_print)))\", \"(root fwd sub_1 Ok(Ok(41))\", \"(root chain (sub_1 fwd sub_1 Ok(Ok(41)))\", \"(root chain (sub_1 fwd add_1 request))\", \"(root chain (sub_1 chain (sub_2 recv print (sub_1 fwd print (root fwd print test-from-framework)))))\", \"(root chain (sub_1 chain (sub_2 add 1 to 42)))\", \"(root chain (sub_1 chain (sub_2 rsp 42 - 1 = Ok(Ok(41)))))\", \"(root fwd add_1 request)\"], to_actor_add_resp: [\"Ok(Ok(43))\"] }", &format!("{:?}", my_context.lock()));
    println!("my_context = {:#?}", my_context);

    // can we access it directly?
    assert_eq!(
        "test_mut_method_data",
        &actor_ref.as_mut().test_mut_method()
    );
}

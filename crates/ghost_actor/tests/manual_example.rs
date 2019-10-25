use std::sync::Arc;

use ghost_actor::prelude::*;
use holochain_tracing::*;

mod manual_example_mod;
#[allow(unused_imports)]
use manual_example_mod::*;

use std::{thread, time::Duration};

#[test]
fn manual_example() {
    // Starts "root" span
    {
        let mut root_span: HSpan = TRACER_SINGLETON
            .lock()
            .unwrap()
            .span("manual_example_root_span")
            .start()
            .into();
        root_span.event("start");
        // SystemTime is not monotonic so wait a bit to make sure following spans are shown after this span
        thread::sleep(Duration::from_millis(1));

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
                    handle_event_to_owner_print: Box::new(
                        |mut span, me: &mut MyContext, message| {
                            span.event(format!("print: \"{}\"", message));
                            me.to_owner_prints.push(message);
                            Ok(())
                        },
                    ),
                    handle_request_to_owner_sub_1: Box::new(
                        |span, _me: &mut MyContext, message, cb| {
                            cb(span.child("sub_1 response"), Ok(message - 1))
                        },
                    ),
                },
            )
            .unwrap();

        {
            let mut span = root_span.child("first event");
            span.set_tag(|| Tag::new("actor", actor_ref.as_mut().name()));
            actor_ref
                .event_to_actor_print(Some(span), "test-from-framework".to_string())
                .unwrap();
        }
        // SystemTime is not monotonic so wait a bit to make sure following spans are shown after this span
        thread::sleep(Duration::from_millis(1));
        {
            let mut span = root_span.child("first request");
            span.set_tag(|| Tag::new("actor", actor_ref.as_mut().name()));
            actor_ref
                .request_to_actor_add_1(
                    Some(span),
                    42,
                    Box::new(|mut span, me, rsp| {
                        span.event(format!("{:?}", rsp));
                        me.to_actor_add_resp.push(format!("{:?}", rsp));
                        Ok(())
                    }),
                )
                .unwrap();
        }
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

        assert_eq!("MyContext { to_owner_prints: [\"(root chain (sub_1 chain (sub_2 to_owner_print)))\", \"(root fwd sub_1 Ok(Ok(41))\", \"(root chain (sub_1 fwd sub_1 Ok(Ok(41)))\", \"(root chain (sub_1 fwd add_1 request))\", \"(root chain (sub_1 chain (sub_2 recv print (sub_1 fwd print (root fwd print test-from-framework)))))\", \"(root chain (sub_1 chain (sub_2 add 1 to 42)))\", \"(root chain (sub_1 chain (sub_2 rsp 42 - 1 = Ok(Ok(41)))))\", \"(root fwd add_1 request)\"], to_actor_add_resp: [\"Ok(Ok(43))\"] }", &format!("{:?}", my_context.lock()));
        println!("\n my_context = {:#?}", my_context);

        // can we access it directly?
        assert_eq!(
            "test_mut_method_data",
            &actor_ref.as_mut().test_mut_method()
        );
    }
    let count = TRACER_SINGLETON.lock().unwrap().drain();
    println!("span count = {}", count);
    TRACER_SINGLETON.lock().unwrap().print(false);
}

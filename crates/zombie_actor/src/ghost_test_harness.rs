/// A test harness for ghost actors. Provides specialized assertion functions
/// to verify predicates have passed, calling the GhostActor or GhostCanTrack process function as many
/// times a necessary until success (up to a hard coded number of iterations, currently).

/// Waits for work to be done. Will interrupt the program if no work was done and should_abort
/// is true
#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_did_work {
    ($ghost_actor: ident,
     $should_abort: expr
    ) => {{
        let timeout = std::time::Duration::from_millis(2000);
        $crate::wait_did_work!($ghost_actor, $should_abort, timeout)
    }};
    ($ghost_actor:ident) => {
        $crate::wait_did_work!($ghost_actor, true)
    };
    ($ghost_actor: ident,
     $should_abort: expr,
     $timeout : expr
      ) => {{
        let mut did_work = false;
        let clock = std::time::SystemTime::now();

        for i in 0..20 {
            did_work = $ghost_actor
                .process()
                .map_err(|e| error!("ghost actor processing error: {:?}", e))
                .map(|work_was_done| work_was_done.into())
                .unwrap_or(did_work);
            if did_work {
                break;
            }
            let elapsed = clock.elapsed().unwrap();
            if elapsed > $timeout {
                break;
            }
            trace!("[{}] wait_did_work", i);
            std::thread::sleep(std::time::Duration::from_millis(1))
        }
        if $should_abort {
            assert!(did_work);
        }
        did_work
    }};
}

/// Waits until a GhostCanTrack process has been invoked and work was done.
#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_can_track_did_work {
    ($ghost_can_track: ident,
     $user_data: expr,
     $should_abort: expr
    ) => {{
        let duration = std::time::Duration::from_millis(2000);
        $crate::wait_can_track_did_work!($ghost_can_track, $user_data, $should_abort, duration)
    }};
    ($ghost_can_track: ident,
     $user_data: expr
    ) => {
        wait_can_track_did_work!($ghost_can_track, $user_data, true)
    };
    ($ghost_can_track: ident,
     $user_data: expr,
     $should_abort: expr,
     $timeout: expr
    ) => {{
        let mut did_work = false;
        let clock = std::time::SystemTime::now();
        for i in 0..20 {
            did_work = $ghost_can_track
                .process(&mut $user_data)
                .map_err(|e| error!("ghost actor processing error: {:?}", e))
                .map(|work_was_done| work_was_done.into())
                .unwrap_or(did_work);
            if did_work {
                break;
            }
            let elapsed = clock.elapsed().unwrap();
            if elapsed > $timeout {
                break;
            }
            trace!("[{}] wait_did_work", i);
            std::thread::sleep(std::time::Duration::from_millis(1))
        }
        if $should_abort {
            assert!(did_work);
        }
        did_work
    }};
}

/// Continues processing the GhostActor trait until no work is being done.
#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_until_no_work {
    ($ghost_actor: ident) => {{
        let mut did_work;
        loop {
            did_work = $crate::wait_did_work!($ghost_actor, false);
            if !did_work {
                break;
            }
        }
        did_work
    }};
    ($ghost_can_track: ident, $user_data: ident) => {{
        let mut did_work;
        loop {
            did_work = $crate::wait_can_track_did_work!($ghost_can_track, $user_data, false);
            if !did_work {
                break;
            }
        }
        did_work
    }};
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_for_messages {
    ($ghost_actors: expr, $endpoint: ident, $user_data: expr, $regexes: expr) => {{
        $crate::wait_for_messages!($ghost_actors, $endpoint, $user_data, $regexes, 5000, true)
    }};
    ($ghost_actors: expr, $endpoint: ident, $user_data: expr, $regexes: expr, $timeout_ms: expr) => {{
        $crate::wait_for_messages!(
            $ghost_actors,
            $endpoint,
            $user_data,
            $regexes,
            $timeout_ms,
            true
        )
    }};
    (
        $ghost_actors: expr,
        $endpoint: ident,
        $user_data: expr,
        $regexes: expr,
        $timeout_ms: expr,
        $should_abort: expr
    ) => {{
        let mut tries = 0;

        let mut message_regexes: Vec<regex::Regex> = $regexes
            .into_iter()
            .map(|re| {
                regex::Regex::new(re)
                    .expect(format!("Regex must be syntactically correct: {:?}", re).as_str())
            })
            .collect();

        let POLL_INTERVAL = 2;
        let mut actors = $ghost_actors;
        loop {
            tries += 1;
            std::thread::sleep(std::time::Duration::from_millis(POLL_INTERVAL));
            actors = actors
                .into_iter()
                .map(|mut actor| {
                    let _ = $crate::wait_did_work!(actor, false);
                    actor
                })
                .collect::<Vec<_>>();
            let _ = $endpoint.process(&mut $user_data);
            for mut message in $endpoint.drain_messages() {
                let message_regexes2 = message_regexes.clone();
                message_regexes = message
                    .take_message()
                    .map(|message| {
                        let message_string = &format!("{:?}", message);
                        trace!("[wait_for_messsage] drained {:?}", message_string);
                        message_regexes
                            .into_iter()
                            .filter(|message_regex| !message_regex.is_match(message_string))
                            .collect()
                    })
                    .unwrap_or(message_regexes2);
                if message_regexes.is_empty() {
                    break;
                }
            }

            if message_regexes.is_empty() || tries > $timeout_ms / POLL_INTERVAL {
                break;
            }
        }
        let is_empty = message_regexes.is_empty();

        if $should_abort {
            assert!(
                is_empty,
                "Did not receive a message matching the provided regexes"
            );
        }
        is_empty
    }};
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_for_message {
    ($ghost_actors: expr, $endpoint: ident, $user_data: expr, $regex: expr) => {{
        $crate::wait_for_message!($ghost_actors, $endpoint, $user_data, $regex, 5000, true)
    }};
    ($ghost_actors: expr, $endpoint: ident, $user_data: expr, $regexes: expr, $timeout_ms: expr) => {{
        $crate::wait_for_message!(
            $ghost_actors,
            $endpoint,
            $user_data,
            $regex,
            $timeout_ms,
            true
        )
    }};
    (
        $ghost_actors: expr,
        $endpoint: ident,
        $user_data: expr,
        $regex: expr,
        $timeout_ms: expr,
        $should_abort: expr
    ) => {{
        let regexes = vec![$regex];
        $crate::wait_for_messages!(
            $ghost_actors,
            $endpoint,
            $user_data,
            regexes,
            $timeout_ms,
            $should_abort
        )
    }};
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! wait1_for_message {
    ($ghost_actor: expr, $endpoint: ident, $regex: expr) => {{
        let actors = vec![&mut $ghost_actor];
        $crate::wait_for_message!(actors, $endpoint, $regex)
    }};
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! wait1_for_messages {
    ($ghost_actor: expr, $endpoint: ident, $user_data: expr, $regexes: expr) => {{
        let actors = vec![&mut $ghost_actor];
        $crate::wait_for_messages!(actors, $endpoint, $user_data, $regexes)
    }};
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! wait1_for_callback {
    ($actor: ident, $ghost_can_track: ident, $request: expr, $re: expr) => {{
        let regex = regex::Regex::new($re.clone())
            .expect(format!("[wait1_for_callback] invalid regex: {:?}", $re).as_str());

        let mut user_data = None;

        let f: GhostCallback<Option<String>, _, _> = Box::new(|user_data, cb_data| {
            user_data.replace(format!("{:?}", cb_data).to_string());
            Ok(())
        });

        $ghost_can_track
            .request(
                holochain_tracing::test_span("wait1_for_callback"),
                $request,
                f,
            )
            .unwrap();

        let MAX_ITERS = 20;
        let mut work_to_do = true;
        for _iter in 0..MAX_ITERS {
            work_to_do |= $crate::wait_until_no_work!($actor);
            work_to_do |= $crate::wait_until_no_work!($ghost_can_track, user_data);
            if !work_to_do {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1))
        }

        let actual = user_data.unwrap_or("Callback not triggered".to_string());

        let is_match = regex.is_match(actual.as_str());

        if is_match {
            assert!(is_match);
        } else {
            assert_eq!($re, actual.as_str());
        }
    }};
}

#[cfg(test)]
mod tests {

    use crate::{GhostResult, WorkWasDone};

    #[derive(Debug, Clone, PartialEq)]
    struct DidWorkActor(i8);

    /// Minimal actor stub that considers work done until counter reaches zero
    impl DidWorkActor {
        pub fn process(&mut self) -> GhostResult<WorkWasDone> {
            if self.0 == 0 {
                Ok(false.into())
            } else if self.0 > 0 {
                self.0 -= 1;
                Ok(true.into())
            } else {
                self.0 += 1;
                Ok(false.into())
            }
        }
    }

    struct DidWorkParentWrapper;
    impl DidWorkParentWrapper {
        pub fn process(&mut self, user_data: &mut DidWorkActor) -> GhostResult<WorkWasDone> {
            user_data.process()
        }
    }

    #[test]
    fn test_wait_did_work() {
        let actor = &mut DidWorkActor(1);

        wait_did_work!(actor);

        assert_eq!(false, wait_did_work!(actor, false));
    }

    #[test]
    fn test_wait_did_work_timeout() {
        let actor = &mut DidWorkActor(-1);

        let timeout = std::time::Duration::from_millis(0);
        let did_work: bool = wait_did_work!(actor, false, timeout);
        assert_eq!(false, did_work);
    }

    #[test]
    fn test_wait_can_track_did_work_timeout() {
        let parent = &mut DidWorkParentWrapper;
        let mut actor = &mut DidWorkActor(-1);

        let timeout = std::time::Duration::from_millis(0);
        let did_work: bool = wait_can_track_did_work!(parent, actor, false, timeout);
        assert_eq!(false, did_work);
    }

    #[test]
    fn test_wait_can_track_did_work() {
        let parent = &mut DidWorkParentWrapper;
        let mut actor = &mut DidWorkActor(1);
        wait_can_track_did_work!(parent, actor);

        assert_eq!(false, wait_can_track_did_work!(parent, actor, false));
    }

    #[test]
    fn test_wait_until_no_work() {
        let actor = &mut DidWorkActor(2);

        wait_until_no_work!(actor);

        assert_eq!(false, wait_did_work!(actor, false));
    }

    #[test]
    fn test_wait_can_track_until_no_work() {
        let parent = &mut DidWorkParentWrapper;
        let mut actor = &mut DidWorkActor(2);
        wait_until_no_work!(parent, actor);

        assert_eq!(false, wait_can_track_did_work!(parent, actor, false));
    }

}

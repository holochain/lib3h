/// A test harness for ghost actors. Provides specialized assertion functions
/// to verify predicates have passed, calling the GhostActor or GhostCanTrack process function as many
/// times a necessary until success (up to a hard coded number of iterations, currently).

/// Waits for work to be done. Will interrupt the program if no work was done and should_abort
/// is true
///
///

pub const DEFAULT_MAX_ITERS: u64 = 20;
pub const DEFAULT_MAX_RETRIES: u64 = 5;
pub const DEFAULT_DELAY_INTERVAL_MS: u64 = 1;
pub const DEFAULT_TIMEOUT_MS: u64 = 2000;
pub const DEFAULT_SHOULD_ABORT: bool = true;
pub const DEFAULT_WAIT_DID_WORK_MAX_ITERS: u64 = 5;
pub const DEFAULT_WAIT_DID_WORK_TIMEOUT_MS: u64 = 5;

/// All configurable parameters when processing an actor.
#[derive(Clone, Debug)]
pub struct ProcessingOptions {
    pub max_iters: u64,
    pub max_retries: u64,
    pub delay_interval_ms: u64,
    pub timeout_ms: u64,
    pub should_abort: bool,
}

impl ProcessingOptions {
    pub fn wait_did_work_defaults() -> Self {
        Self {
            max_iters: DEFAULT_WAIT_DID_WORK_MAX_ITERS,
            timeout_ms: DEFAULT_WAIT_DID_WORK_TIMEOUT_MS,
            ..Default::default()
        }
    }

    pub fn with_should_abort(should_abort: bool) -> Self {
        let options = Self {
            should_abort,
            ..Default::default()
        };
        options
    }
}

impl Default for ProcessingOptions {
    fn default() -> Self {
        Self {
            max_iters: DEFAULT_MAX_ITERS,
            max_retries: DEFAULT_MAX_RETRIES,
            delay_interval_ms: DEFAULT_DELAY_INTERVAL_MS,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            should_abort: DEFAULT_SHOULD_ABORT,
        }
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_did_work {
    ($ghost_actor:ident) => {{
        let options = $crate::ghost_test_harness::ProcessingOptions::wait_did_work_defaults();
        $crate::wait_did_work!($ghost_actor, options);
    }};
    ($ghost_actor: ident,
     $options: expr
    ) => {{
        let mut did_work = false;
        let clock = std::time::SystemTime::now();

        let timeout = std::time::Duration::from_millis($options.timeout_ms);

        for i in 0..$options.max_iters {
            did_work = $ghost_actor
                .process()
                .map_err(|e| error!("ghost actor processing error: {:?}", e))
                .map(|work_was_done| work_was_done.into())
                .unwrap_or(did_work);
            if did_work {
                break;
            }
            let elapsed = clock.elapsed().unwrap();
            if elapsed > timeout {
                trace!("[epoch {}] wait_did_work timeout", i);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis($options.delay_interval_ms))
        }
        if $options.should_abort {
            assert!(did_work);
        }
        trace!("wait_did_work returning: {:?}", did_work);

        did_work
    }};
}

/// Waits until a GhostCanTrack process has been invoked and work was done.
#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_can_track_did_work {
    ($ghost_can_track: ident,
     $user_data: expr
    ) => {
        let options = $crate::ghost_test_harness::ProcessingOptions::wait_did_work_defaults();
        wait_can_track_did_work!($ghost_can_track, $user_data, options)
    };
    ($ghost_can_track: ident,
     $user_data: expr,
     $options: expr
    ) => {{
        let mut did_work = false;
        let clock = std::time::SystemTime::now();
        let timeout = std::time::Duration::from_millis($options.timeout_ms);
        for i in 0..$options.max_iters {
            did_work = $ghost_can_track
                .process(&mut $user_data)
                .map_err(|e| error!("ghost actor processing error: {:?}", e))
                .map(|work_was_done| work_was_done.into())
                .unwrap_or(did_work);
            if did_work {
                break;
            }
            let elapsed = clock.elapsed().unwrap();
            if elapsed > timeout {
                trace!("[{}] wait_can_track_did_work timeout", i);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis($options.delay_interval_ms))
        }
        if $options.should_abort {
            assert!(did_work);
        }
        trace!("wait_can_track_did_work returning {:?}", did_work);

        did_work
    }};
}

/// Continues processing the GhostActor or GhostCanTrack trait
/// until no work is being done.
#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_until_no_work {
    ($ghost_actor: ident) => {{
        let mut did_work = false;
        let options = $crate::ghost_test_harness::ProcessingOptions::with_should_abort(false);

        let wait_options = $crate::ghost_test_harness::ProcessingOptions {
            max_iters: $crate::ghost_test_harness::DEFAULT_WAIT_DID_WORK_MAX_ITERS,
            timeout_ms: $crate::ghost_test_harness::DEFAULT_WAIT_DID_WORK_TIMEOUT_MS,
            ..options
        };

        
        let clock = std::time::SystemTime::now();

        let timeout = std::time::Duration::from_millis(options.timeout_ms);

 
        for i in 0..options.max_iters {
            did_work = $crate::wait_did_work!($ghost_actor, wait_options);

            if !did_work {
                break;
            }
            
            let elapsed = clock.elapsed().unwrap();
            if elapsed > timeout {
                trace!("[epoch {}] wait_until_no_work timeout", i);
                break;
            }
         }
        did_work
    }};
    ($ghost_can_track: ident, $user_data: ident) => {{
        let mut did_work = false;
        let options = $crate::ghost_test_harness::ProcessingOptions::with_should_abort(false);
        let wait_options = $crate::ghost_test_harness::ProcessingOptions {
            max_iters: $crate::ghost_test_harness::DEFAULT_WAIT_DID_WORK_MAX_ITERS,
            timeout_ms: $crate::ghost_test_harness::DEFAULT_WAIT_DID_WORK_TIMEOUT_MS,
            ..options
        };
 
        let clock = std::time::SystemTime::now();

        let timeout = std::time::Duration::from_millis(options.timeout_ms);

        for i in 0..options.max_iters {
            did_work = $crate::wait_can_track_did_work!($ghost_can_track, $user_data, wait_options);
            if !did_work {
                break;
            }
            let elapsed = clock.elapsed().unwrap();
            if elapsed > timeout {
                trace!("[epoch {}] wait_until_no_work timeout", i);
                break;
            }

        }

        did_work
    }};
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_for_messages {
    ($ghost_actors: expr,
     $endpoint: ident,
     $user_data: expr,
     $regexes: expr) => {{
        let options: $crate::ghost_test_harness::ProcessingOptions = Default::default();
        $crate::wait_for_messages!($ghost_actors, $endpoint, $user_data, $regexes, options)
    }};
    (
        $ghost_actors: expr,
        $endpoint: ident,
        $user_data: expr,
        $regexes: expr,
        $options: expr
    ) => {{
        let mut message_regexes: Vec<regex::Regex> = $regexes
            .into_iter()
            .map(|re| {
                regex::Regex::new(re)
                    .expect(format!("Regex must be syntactically correct: {:?}", re).as_str())
            })
            .collect();

        let mut actors = $ghost_actors;
        for tries in 0..$options.max_iters {
            std::thread::sleep(std::time::Duration::from_millis($options.delay_interval_ms));
            actors = actors
                .into_iter()
                .map(|mut actor| {
                    let _ = $crate::wait_until_no_work!(actor);
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
                        trace!("[wait_for_messsages] drained {:?}", message_string);
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

            if message_regexes.is_empty()
                || tries > $options.timeout_ms / $options.delay_interval_ms
            {
                break;
            }
        }
        let is_empty = message_regexes.is_empty();

        if $options.should_abort {
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
        let options: $crate::ghost_test_harness::ProcessingOptions = Default::default();
        $crate::wait_for_message!($ghost_actors, $endpoint, $user_data, $regex, options)
    }};
    (
        $ghost_actors: expr,
        $endpoint: ident,
        $user_data: expr,
        $regex: expr,
        $options: expr
    ) => {{
        let regexes = vec![$regex];
        $crate::wait_for_messages!($ghost_actors, $endpoint, $user_data, regexes, $options)
    }};
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! wait1_for_message {
    ($ghost_actor: expr, $endpoint: ident, $regex: expr) => {{
        let actors = vec![&mut $ghost_actor];
        $crate::wait_for_message!(actors, $endpoint, $regex)
    }};
    ($ghost_actor: expr, $endpoint: ident, $regex: expr, $options: expr) => {{
        let actors = vec![&mut $ghost_actor];
        $crate::wait_for_message!(actors, $endpoint, $regex, $options)
    }};
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! wait1_for_messages {
    ($ghost_actor: expr, $endpoint: ident, $user_data: expr, $regexes: expr) => {{
        let actors = vec![&mut $ghost_actor];
        $crate::wait_for_messages!(actors, $endpoint, $user_data, $regexes)
    }};
    ($ghost_actor: expr, $endpoint: ident, $user_data: expr, $regexes: expr, $options: expr) => {{
        let actors = vec![&mut $ghost_actor];
        $crate::wait_for_messages!(actors, $endpoint, $user_data, $regexes, $options)
    }};
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! wait1_for_callback {
    ($actor: ident, $ghost_can_track: ident, $request: expr, $re: expr) => {{
        let options: $crate::ghost_test_harness::ProcessingOptions = Default::default();
        $crate::wait1_for_callback!($actor, $ghost_can_track, $request, $re, options)
    }};
    ($actor: ident, $ghost_can_track: ident, $request: expr, $re: expr, $options: expr) => {{
        let regex = regex::Regex::new($re.clone())
            .expect(format!("[wait1_for_callback] invalid regex: {:?}", $re).as_str());

        let mut user_data = None;

        let f: $crate::GhostCallback<Option<String>, _, _> = Box::new(|user_data, cb_data| {
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

        let mut work_to_do = true;
        for iter in 0..$options.max_iters {
            work_to_do |= $crate::wait_until_no_work!($actor);
            work_to_do |= $crate::wait_until_no_work!($ghost_can_track, user_data);
            if !work_to_do {
                break;
            }

            if iter > $options.timeout_ms / $options.delay_interval_ms {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis($options.delay_interval_ms))
        }

        let actual = user_data.unwrap_or("Callback not triggered".to_string());

        let is_match = regex.is_match(actual.as_str());

        if $options.should_abort {
            if is_match {
                assert!(is_match);
            } else {
                assert_eq!($re, actual.as_str());
            }
        }
        is_match
    }};
}

/// Similar to `wait1_for_callback!` but will invoke the callback multiple times until
/// success. Users need provide a closure `$request_fn` that takes a user defined state and
/// produces a triple of `(request_to_other, regex, new_state)`. If the request fails to produce
/// the matching regex, the closure will be invoked again with `new_state` instead of `state`.
/// This will continue until success or a finite number of failures has been reached.
///
/// If `$should_abort` is `false`, the function returns a tuple ``(is_match, final_state)` where
/// `is_match` indicates whether the regexed match and `final_state` contains the final state produced
/// by the closure.
#[allow(unused_macros)]
#[macro_export]
macro_rules! wait1_for_repeatable_callback {
    ($actor: ident, $ghost_can_track: ident, $request_fn: expr, $init_value: expr) => {{
        $crate::wait1_for_repeatable_callback!(
            $actor,
            $ghost_can_track,
            $request_fn,
            $init_value,
            $crate::ghost_test_harness::ProcessingOptions::default()
        )
    }};
    ($actor: ident, $ghost_can_track: ident, $request_fn: expr, $init_value: expr, $options: expr) => {{
        let mut is_match = false;

        let mut state = $init_value;

        for iter in 0..$options.max_retries {
            let (request, re, state_prime) = ($request_fn)(state);
            state = state_prime;
            let should_abort = $options.should_abort && iter == $options.max_retries;
            let wait_options = $crate::ghost_test_harness::ProcessingOptions {
                should_abort: should_abort,
                ..$options
            };
            is_match = $crate::wait1_for_callback!(
                $actor,
                $ghost_can_track,
                request,
                re.as_str(),
                wait_options
            );
            if is_match {
                break;
            }
        }
        (is_match, state)
    }};
}

#[cfg(test)]
mod tests {

    use super::ProcessingOptions;
    use crate::{GhostCallback, GhostCallbackData, GhostResult, WorkWasDone};
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

    #[derive(Debug, Clone)]
    pub enum RequestToOther {
        Ping,
        Retry,
    }

    #[derive(Debug, Clone)]
    pub enum RequestToOtherResponse {
        Pong,
        Retry,
    }

    #[derive(Debug)]
    struct DidWorkParentWrapper;
    impl DidWorkParentWrapper {
        pub fn process(&mut self, actor: &mut DidWorkActor) -> GhostResult<WorkWasDone> {
            actor.process()
        }
    }

    pub type CallbackError = String;
    pub type Callback = GhostCallback<Option<String>, RequestToOtherResponse, CallbackError>;
    #[allow(dead_code)]
    pub type CallbackData = GhostCallbackData<RequestToOtherResponse, CallbackError>;

    struct CallbackParentWrapper(pub Vec<(Callback, CallbackData)>);

    pub type CallbackUserData = Option<String>;

    impl CallbackParentWrapper {
        pub fn request(
            &mut self,
            _span: holochain_tracing::Span,
            payload: RequestToOther,
            cb: Callback,
        ) -> GhostResult<()> {
            let response = match payload {
                RequestToOther::Ping => RequestToOtherResponse::Pong,
                RequestToOther::Retry => RequestToOtherResponse::Retry,
            };

            let cb_data = GhostCallbackData::Response(Ok(response));
            self.0.push((cb, cb_data));
            Ok(())
        }

        pub fn process(
            &mut self,
            mut user_data: &mut CallbackUserData,
        ) -> GhostResult<WorkWasDone> {
            if let Some((cb, cb_data)) = self.0.pop() {
                let _cb_result = (cb)(&mut user_data, cb_data);
                Ok(true.into())
            } else {
                Ok(false.into())
            }
        }

        pub fn new() -> Self {
            CallbackParentWrapper(vec![])
        }
    }

    #[test]
    fn test_wait_did_work() {
        let actor = &mut DidWorkActor(1);

        wait_did_work!(actor);

        assert_eq!(
            false,
            wait_did_work!(actor, ProcessingOptions::with_should_abort(false))
        );
    }

    #[test]
    fn test_wait_did_work_timeout() {
        let actor = &mut DidWorkActor(-1);

        let options = ProcessingOptions {
            should_abort: false,
            timeout_ms: 0,
            ..Default::default()
        };
        let did_work: bool = wait_did_work!(actor, options);
        assert_eq!(false, did_work);
    }

    #[test]
    fn test_wait_can_track_did_work_timeout() {
        let parent = &mut DidWorkParentWrapper;
        let mut actor = &mut DidWorkActor(-1);

        let options = ProcessingOptions {
            should_abort: false,
            timeout_ms: 0,
            ..Default::default()
        };
        let did_work: bool = wait_can_track_did_work!(parent, actor, options);
        assert_eq!(false, did_work);
    }

    #[test]
    fn test_wait_can_track_did_work() {
        let parent = &mut DidWorkParentWrapper;
        let mut actor = &mut DidWorkActor(1);
        wait_can_track_did_work!(parent, actor);

        assert_eq!(
            false,
            wait_can_track_did_work!(parent, actor, ProcessingOptions::with_should_abort(false))
        );
    }

    #[test]
    fn test_wait_until_no_work() {
        let actor = &mut DidWorkActor(2);

        wait_until_no_work!(actor);

        assert_eq!(
            false,
            wait_did_work!(actor, ProcessingOptions::with_should_abort(false))
        );
    }

    #[test]
    fn test_wait_can_track_until_no_work() {
        let parent = &mut DidWorkParentWrapper;
        let mut actor = &mut DidWorkActor(2);
        wait_until_no_work!(parent, actor);

        assert_eq!(
            false,
            wait_can_track_did_work!(parent, actor, ProcessingOptions::with_should_abort(false))
        );
    }

    #[test]
    fn test_wait_for_callback() {
        let parent = &mut CallbackParentWrapper::new();
        let actor = &mut DidWorkActor(1);

        let request = RequestToOther::Ping;
        let is_match = wait1_for_callback!(
            actor,
            parent,
            request,
            "Pong",
            ProcessingOptions::with_should_abort(false)
        );
        assert!(is_match);

        let request = RequestToOther::Retry;
        let is_match = wait1_for_callback!(
            actor,
            parent,
            request,
            "Pong",
            ProcessingOptions::with_should_abort(false)
        );
        assert!(!is_match);
    }

    #[test]
    fn test_wait_for_repeatable_callback() {
        let parent = &mut CallbackParentWrapper::new();
        let actor = &mut DidWorkActor(1);

        let request_fn = Box::new(|retried| {
            if retried {
                // Test should succeed with these inputs
                (RequestToOther::Ping, "Pong".to_string(), true)
            } else {
                // Purposely cause the callback to be triggered again
                (RequestToOther::Retry, "Pong".to_string(), true)
            }
        });

        let (is_match, retried) = wait1_for_repeatable_callback!(
            actor,
            parent,
            request_fn,
            false,
            ProcessingOptions::with_should_abort(false)
        );
        assert!(is_match);
        assert!(retried);
    }

}

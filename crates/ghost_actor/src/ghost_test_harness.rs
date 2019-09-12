/// A test harness for ghost actors. Provides specialized assertion functions
/// to verify predicates have passed, calling the ghost_actor process function as many
/// times a necessary until success (up to a hard coded number of iterations, currently).
use predicates::prelude::*;
use crate::GhostCallbackData;

/// Represents all useful state after a single call to an ghost_actor's process function
#[derive(Debug)]
pub struct ProcessorResult<UserData, Cb:'static, E:'static> {
    /// Whether the ghost_actor reported doing work or not
    pub did_work: bool,
    /// All events produced by the last call to process for ghost_actor
    pub callback_data : GhostCallbackData<Cb, E>,
    /// All previously processed results
    pub previous: Vec<ProcessorResult<UserData, Cb, E>>,
    pub user_data : UserData
}

/// An assertion style processor which provides a
/// predicate over ProcessorResult (the eval function) and a
/// test function which will break control flow similar to
/// how calling assert! or assert_eq! would.
pub trait Processor<UserData, Cb:'static, E:'static>: Predicate<ProcessorResult<UserData, Cb, E>> {
    /// Processor name, for debugging and mapping purposes
    fn name(&self) -> String {
        "default_processor".into()
    }

    /// Test the predicate function. Should interrupt control
    /// flow with a useful error if self.eval(args) is false.
    fn test(&self, args: &ProcessorResult<UserData, Cb, E>);
}

/// Asserts some extracted data from ProcessorResult is equal to an expected instance.
pub trait AssertEquals<UserData, Cb:'static, E:'static, T: PartialEq + std::fmt::Debug> {
    /// User defined function for extracting a collection data of a specific
    /// type from the proessor arguments
    fn extracted(&self, args: &ProcessorResult<UserData, Cb, E>) -> Option<T>;

    /// The expected value to compare to the actual value to
    fn expected(&self) -> &T;
}

impl<UserData, Cb:'static, E:'static, T: PartialEq + std::fmt::Debug> std::fmt::Display for dyn AssertEquals<UserData, Cb, E, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", "assert_equals")
    }
}

impl<UserData, Cb:'static, E:'static, T> predicates::reflection::PredicateReflection for dyn AssertEquals<UserData, Cb, E, T> where
    T: PartialEq + std::fmt::Debug
{
}

impl<UserData, Cb:'static, E:'static, T> Predicate<ProcessorResult<UserData, Cb, E>> for dyn AssertEquals<UserData, Cb, E, T>
where
    T: PartialEq + std::fmt::Debug,
{
    fn eval(&self, args: &ProcessorResult<UserData, Cb, E>) -> bool {
        self.extracted(args)
            .map(|actual| &actual == self.expected())
            .unwrap_or(false)
    }
}

/// Asserts some extracted data from ProcessorResult passes a predicate.  
pub trait Assert<UserData, Cb:'static, E:'static, T> {
    fn extracted(&self, args: &ProcessorResult<UserData, Cb, E>) -> Option<T>;

    fn assert_inner(&self, args: &T) -> bool;
}

/// Asserts that the actual is equal to the given expected
#[derive(PartialEq, Debug)]
pub struct CallbackDataEquals<Cb>(pub Cb);

impl<UserData, Cb, E:'static> predicates::Predicate<ProcessorResult<UserData, Cb, E>> for CallbackDataEquals<Cb>
where
    Cb: PartialEq + std::fmt::Debug + 'static
{
    fn eval(&self, args: &ProcessorResult<UserData, Cb, E>) -> bool {
        self.extracted(args)
            .map(|actual| &actual == self.expected())
            .unwrap_or(false)
    }
}

impl<UserData, Cb, E:'static> AssertEquals<UserData, Cb, E, Cb> for CallbackDataEquals<Cb> 
where
    Cb: PartialEq + std::fmt::Debug + 'static
{
    fn extracted(&self, args: &ProcessorResult<UserData, Cb, E>) -> Option<Cb> {
        match args.callback_data {
            GhostCallbackData::Timeout => None, 
            GhostCallbackData::Response(Err(_err)) => None,
            GhostCallbackData::Response(Ok(cb)) => Some(cb) 
        } 
    }
    fn expected(&self) -> &Cb {
        &self.0
    }
}

impl<Cb> std::fmt::Display for CallbackDataEquals<Cb>
where
    Cb: PartialEq + std::fmt::Debug + 'static {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<UserData, Cb, E:'static> Processor<UserData, Cb, E> 
    for CallbackDataEquals<Cb> where Cb : std::fmt::Debug + 'static + PartialEq {
    fn test(&self, args: &ProcessorResult<UserData, Cb, E>) {
        let actual = self.extracted(args);
        assert_eq!(Some(*self.expected()), actual);
    }

    fn name(&self) -> String {
        format!("{:?}", self).to_string()
    }
}

impl<Cb> predicates::reflection::PredicateReflection for CallbackDataEquals<Cb> 
where Cb : std::fmt::Debug + 'static + PartialEq {}


/// Asserts using an arbitrary predicate over a lib3h server protocol event
pub struct CallbackDataAssert<Cb>(pub Box<dyn Predicate<Cb>>);

impl<UserData, Cb:'static, E:'static> Assert<UserData, Cb, E, Cb> for CallbackDataAssert<Cb> {
    fn extracted(&self, args: &ProcessorResult<UserData, Cb, E>) -> Option<Cb> {
        match args.callback_data {
            GhostCallbackData::Timeout => None, 
            GhostCallbackData::Response(Err(_err)) => None,
            GhostCallbackData::Response(Ok(cb)) => Some(cb) 
        } 
    }

    fn assert_inner(&self, cb: &Cb) -> bool {
        self.0.eval(&cb)
    }
}


impl<UserData, Cb:'static, E:'static> Processor<UserData, Cb, E> for CallbackDataAssert<Cb> {
    fn test(&self, args: &ProcessorResult<UserData, Cb, E>) {
        let actual = self.extracted(args);

        if let Some(actual) = actual {
            assert!(self.assert_inner(&actual));
        } else {
            assert!(actual.is_some());
        }
    }

    fn name(&self) -> String {
        "CallbackDataAssert".to_string()
    }
}

impl<UserData, Cb:'static, E:'static> Predicate<ProcessorResult<UserData, Cb, E>> for CallbackDataAssert<Cb> {
    fn eval(&self, args: &ProcessorResult<UserData, Cb, E>) -> bool {
        self.extracted(args)
            .map(|actual| self.assert_inner(&actual))
            .unwrap_or(false)
    }
}

impl<Cb> std::fmt::Display for CallbackDataAssert<Cb> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", "callback data assertion")
    }
}

impl<Cb> predicates::reflection::PredicateReflection for CallbackDataAssert<Cb> {}

/// Asserts work was done
#[derive(PartialEq, Debug)]
pub struct DidWorkAssert;

impl<UserData, Cb:'static, E:'static> Processor<UserData, Cb, E> for DidWorkAssert {
    fn test(&self, args: &ProcessorResult<UserData, Cb, E>) {
        assert!(args.did_work);
    }

    fn name(&self) -> String {
        format!("{:?}", self).to_string()
    }
}

impl<UserData, Cb:'static, E:'static> Predicate<ProcessorResult<UserData, Cb, E>> for DidWorkAssert {
    fn eval(&self, args: &ProcessorResult<UserData, Cb, E>) -> bool {
        args.did_work
    }
}

impl std::fmt::Display for DidWorkAssert {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?} did work", self.name())
    }
}

impl predicates::reflection::PredicateReflection for DidWorkAssert {}

#[allow(unused_macros)]
/// Convenience function that asserts only one particular equality predicate
/// passes for a collection of . See assert_processed for
macro_rules! assert_callback_eq {
    ($ghost_can_track:ident, //: &mumut t Vec<&mut Box<dyn Networkghost_actor>>,
     $user_data:ident,
     $equal_to:ident,// Box<dyn Processor>,
    ) => {{
        let p = Box::new($crate::ghost_test_harness::CallbackDataEqual($equal_to));
        assert_callback_processed!($ghost_can_track, $user_data, p)
    }};
}

#[allow(unused_macros)]
macro_rules! process_one {
    ($ghost_can_track: ident,
     $user_data: ident,
     $previous: ident,
     $events: ident,
     $errors: ident
    ) => {{
        let did_work = $ghost_can_track
            .process(&mut $user_data)
            .map_err(|err| dbg!(err))
            .map(|did_work| did_work.into())
            .unwrap_or(false);
        if !did_work {
        } else {
            let processor_result = $crate::ghost_test_harness::ProcessorResult<_, _, _, _> {
                did_work,
                events : $events
                previous: $previous.clone(),
            };
            let mut failed = Vec::new();

            for (processor, _orig_processor_result) in $errors.drain(..) {
                let result = processor.eval(&processor_result.clone());
                if result {
                    // Simulate the succesful assertion behavior
                    processor.test(&processor_result.clone());
                // processor passed!
                } else {
                    // Cache the assertion error and trigger it later if we never
                    // end up passing
                    failed.push((processor, Some(processor_result.clone())));
                }
            }
            $errors.append(&mut failed);
            $previous.push(processor_result.clone());
        }
    }};
}

/// Asserts that a collection of ghost_actors produce events
/// matching a set of predicate functions. For the program
/// to continue executing all processors must pass.
///
/// Multiple calls to process() will be made as needed for
/// the passed in processors to pass. It will failure after
/// MAX_PROCESSING_LOOPS iterations regardless.
///
/// Returns all observed processor results for use by
/// subsequent tests.
#[allow(unused_macros)]
macro_rules! assert_callback_processed {
    ($ghost_can_track:ident,
     $user_data:ident,
     $cb_data:ident,
     $processor:ident
 ) => {
     {
        let mut previous = Vec::new();
        let mut errors: Vec<(
            Box<dyn $crate::ghost_test_harness::Processor<_, _, _, _>>,
            Option<$crate::ghost_test_harness::ProcessorResult<_, _, _, _>>,
        )> = Vec::new();

       let mut events = Vec::new();

       let cb: 
           $crate::ghost_actor::GhostCallback<_, _, _, _> =
           Box::new(|parent, context, callback_data| {
               events.push((parent, context, callback_data));
               Ok(())
           });
   
       let context = lib3h_tracing::TestTrace("assert_callback_processed");

       $ghost_can_track.request(
           context,
           $cb_data
           cb
       ).expect("request to ghost_can_track");

//       for p in vec![$processor] {
           errors.push(($processor, None))
  //     }

        for epoch in 0..20 {
            trace!("[{:?}] {:?}", epoch, previous);

            process_one!($ghost_can_track, $user_data, previous, events, errors);
            if errors.is_empty() {
                break;
            }
    
            events = Vec::new();
        }

        if $should_abort {
            for (p, args) in errors {
                if let Some(args) = args {
                    p.test(&args)
                } else {
                    // Make degenerate result which should fail
                    p.test(&$crate::ghost_test_harness::ProcessorResult {
                        previous: vec![],
                        events: vec![],
                        did_work: false,
                    })
                }
            }
        }
        previous
    }};
}

/// Waits for work to be done. Will interrupt the program if no work was done and should_abort
/// is true
#[allow(unused_macros)]
macro_rules! wait_did_work {
    ($ghost_actor:ident, //&mut Vec<&mut Box<dyn Networkghost_actor>>,
     $should_abort: expr
    ) => {{

        let mut did_work = false;
        for i in 0..20 {
           let did_work = $ghost_actor
               .process()
               .map_err(|e| error!("ghost actor processing error: {:?}", e))
               .map(|work_was_done| work_was_done.into())
               .unwrap_or(false);
            if did_work {
                break;
            }
        }
        if should_abort {
            assert!(did_work);
        }
        return false;
    }};
    ($ghost_actor:ident) => {
        wait_did_work!($ghost_actor, true)
    };
}

/// Continues processing the ghost_actor until no work is being done.
#[allow(unused_macros)]
macro_rules! wait_until_no_work {
    ($ghost_actor: ident) => {{
        let mut did_work;
        loop {
            did_work = wait_did_work!($ghost_actor, false);
            if !did_work {
                break;
            }
        }
        did_work
    }};
}

#[cfg(test)]
mod tests {

}

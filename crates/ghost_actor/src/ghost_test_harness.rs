/// A test harness for ghost actors. Provides specialized assertion functions
/// to verify predicates have passed, calling the ghost_actor process function as many
/// times a necessary until success (up to a hard coded number of iterations, currently).
use predicates::prelude::*;

use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol};

/// Represents all useful state after a single call to an ghost_actor's process function
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ProcessorResult<UserData, Context, Cb, E> {
    /// Whether the ghost_actor reported doing work or not
    pub did_work: bool,
    /// All events produced by the last call to process for ghost_actor
    pub events: Vec<(UserData, Context, GhostCallbackData<Cb, E>)>,
    /// All previously processed results
    pub previous: Vec<ProcessorResult<UserData, Context, Cb, E>>,
}

/// An assertion style processor which provides a
/// predicate over ProcessorResult (the eval function) and a
/// test function which will break control flow similar to
/// how calling assert! or assert_eq! would.
pub trait Processor<UserData, Context, Cb, E>: Predicate<ProcessorResult<UserData, Context, Cb, E>> {
    /// Processor name, for debugging and mapping purposes
    fn name(&self) -> String {
        "default_processor".into()
    }

    /// Test the predicate function. Should interrupt control
    /// flow with a useful error if self.eval(args) is false.
    fn test(&self, args: &ProcessorResult<UserData, Context, Cb, E>);
}

/// Asserts some extracted data from ProcessorResult is equal to an expected instance.
pub trait AssertEquals<UserData, Context, Cb, E, T: PartialEq + std::fmt::Debug> {
    /// User defined function for extracting a collection data of a specific
    /// type from the proessor arguments
    fn extracted(&self, args: &ProcessorResult<UserData, Context, Cb, E>) -> Vec<T>;

    /// The expected value to compare to the actual value to
    fn expected(&self) -> T;
}

impl<UserData, Context, Cb, E, T: PartialEq + std::fmt::Debug> std::fmt::Display for dyn AssertEquals<UserData, Context, Cb, E, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", "assert_equals")
    }
}

impl<UserData, Context, Cb, E, T> predicates::reflection::PredicateReflection for dyn AssertEquals<UserData, Context, Cb, E, T> where
    T: PartialEq + std::fmt::Debug
{
}

impl<UserData, Context, Cb, E, T> Predicate<ProcessorResult<UserData, Context, Cb, E>> for dyn AssertEquals<UserData, Context, Cb, E, T>
where
    T: PartialEq + std::fmt::Debug,
{
    fn eval(&self, args: &ProcessorResult<UserData, Context, Cb, E>) -> bool {
        let extracted = self.extracted(args);
        extracted
            .iter()
            .find(|actual| **actual == self.expected())
            .is_some()
    }
}

/// Asserts some extracted data from ProcessorResult passes a predicate.  
pub trait Assert<UserData, Context, Cb, E, T> {
    fn extracted(&self, args: &ProcessorResult<UserData, Context, Cb, E>) -> Vec<T>;

    fn assert_inner(&self, args: &T) -> bool;
}

/// Asserts that the actual is equal to the given expected
#[allow(dead_code)]
#[derive(PartialEq, Debug)]
pub struct CallbackDataEquals<Cb>(pub Cb);

impl<UserData, Context, Cb, E> predicates::Predicate<ProcessorResult<UserData, Context, Cb, E>> for CallbackDataEquals<Cb> {
    fn eval(&self, args: &ProcessorResult<UserData, Context, Cb, E>) -> bool {
        self.extracted(args)
            .iter()
            .find(|actual| **actual == self.expected())
            .is_some()
    }
}

impl<UserData, Context, Cb, E> AssertEquals<UserData, Context, Cb, E, Cb> for CallbackDataEquals<Cb> {
    fn extracted(&self, args: &ProcessorResult<UserData, Context, Cb, E>) -> Vec<Lib3hServerProtocol> {
        args.events.iter().filter_map(
            |x| { 
                match x { 
                    GhostCallbackData::Timeout => None, 
                    GhostCallbackData::Response(cb) => Some(cb) 
                } 
            }).collect()
    }
    fn expected(&self) -> Cb {
        self.0.clone()
    }
}

impl<Cb> std::fmt::Display for CallbackDataEquals<Cb> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {:?}", self.name(), self)
    }
}

impl<UserData, Context, Cb, E> Processor<UserData, Context, Cb, E> for CallbackDataEquals<Cb> {
    fn test(&self, args: &ProcessorResult<UserData, Context, Cb, E>) {
        let extracted = self.extracted(args);
        let actual = extracted.iter().find(|actual| **actual == self.expected());
        assert_eq!(Some(&self.expected()), actual.or(extracted.first()));
    }

    fn name(&self) -> String {
        format!("{:?}", self).to_string()
    }
}

impl<Cb> predicates::reflection::PredicateReflection for CallbackDataEquals<Cb> {}

/// Asserts using an arbitrary predicate over a lib3h server protocol event
#[allow(dead_code)]
pub struct CallbackDataAssert<Cb>(pub Box<dyn Predicate<Cb>>);

impl<UserData, Context, Cb, E> Assert<UserData, Context, Cb, E> for CallbackDataAssert<Cb> {
    fn extracted(&self, args: &ProcessorResult<UserData, Context, Cb, E>) -> Vec<Cb> {
        args.events.iter().map(|x| x.clone()).collect()
    }

    fn assert_inner(&self, x: &Cb) -> bool {
        self.0.eval(&x)
    }
}


impl<UserData, Context, Cb, E> Processor<UserData, Context, Cb, E> for CallbackDataAssert<Cb> {
    fn test(&self, args: &ProcessorResult<UserData, Context, Cb, E>) {
        let extracted = self.extracted(args);
        let actual = extracted
            .iter()
            .find(move |actual| self.assert_inner(*actual))
            .or(extracted.first());

        if let Some(actual) = actual {
            assert!(self.assert_inner(actual));
        } else {
            assert!(actual.is_some());
        }
    }

    fn name(&self) -> String {
        "CallbackDataAssert".to_string()
    }
}

impl<UserData, Context, Cb, E> Predicate<ProcessorResult<UserData, Context, Cb, E>> for CallbackDataAssert<Cb> {
    fn eval(&self, args: &ProcessorResult<UserData, Context, Cb, E>) -> bool {
        let extracted = self.extracted(args);
        extracted
            .iter()
            .find(|actual| self.assert_inner(*actual))
            .is_some()
    }
}

impl<Cb> std::fmt::Display for CallbackDataAssert<Cb> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {:?}", self.name(), "callback data assertion")
    }
}

impl<Cb> predicates::reflection::PredicateReflection for CallbackDataAssert<Cb> {}

/// Asserts work was done
#[allow(dead_code)]
#[derive(PartialEq, Debug)]
pub struct DidWorkAssert;

impl<UserData, Context, Cb, E> Processor<UserData, Context, Cb, E> for DidWorkAssert {
    fn test(&self, args: &ProcessorResult<UserData, Context, Cb, E>) {
        assert!(args.did_work);
    }

    fn name(&self) -> String {
        format!("{:?}", self).to_string()
    }
}

impl<UserData, Context, Cb, E> Predicate<ProcessorResult<UserData, Context<Cb, E>>> for DidWorkAssert {
    fn eval(&self, args: &ProcessorResult<UserData, Context, Cb, E>) -> bool {
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
     $equal_to:ident,// Box<dyn Processor>,
    ) => {{
        let p = Box::new($crate::ghost_test_harness::CallbackDataEqual($equal_to));
        assert_processed!($ghost_can_track, p)
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
macro_rules! assert_processed {
    ($ghost_can_track:ident,
     $user_data:ident,
     $context:ident,
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
    
       $ghost_can_track.request(
           $context,
           $cb_data
           cb
       ).expect("request to ghost_actor1");

//       for p in vec![$processor] {
           errors.push(($processor, None))
  //     }

        for epoch in 0..20 {
            println!("[{:?}] {:?}", epoch, previous);

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


mod tests {
    

}

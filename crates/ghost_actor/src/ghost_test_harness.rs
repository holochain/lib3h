/// A test harness for ghost actors. Provides specialized assertion functions
/// to verify predicates have passed, calling the ghost_actor process function as many
/// times a necessary until success (up to a hard coded number of iterations, currently).
use predicates::prelude::*;
use crate::GhostCallbackData;

/// Represents all useful state after a single call to an ghost_actor's process function
#[derive(Debug)]
pub struct ProcessorResult<Cb:'static, E:'static> {
    /// Whether the ghost_actor reported doing work or not
    pub did_work: bool,
    /// All events produced by the last call to process for ghost_actor
    pub callback_data : Option<GhostCallbackData<Cb, E>>,
    /// All previously processed results
    pub previous: Vec<ProcessorResult<Cb, E>>,
//    pub user_data : UserData
}

/// An assertion style processor which provides a
/// predicate over ProcessorResult (the eval function) and a
/// test function which will break control flow similar to
/// how calling assert! or assert_eq! would.
pub trait Processor<Cb:'static, E:'static>: Predicate<ProcessorResult<Cb, E>> {
    /// Processor name, for debugging and mapping purposes
    fn name(&self) -> String {
        "default_processor".into()
    }

    /// Test the predicate function. Should interrupt control
    /// flow with a useful error if self.eval(args) is false.
    fn test(&self, args: &ProcessorResult<Cb, E>);
}

/// Asserts some extracted data from ProcessorResult is equal to an expected instance.
pub trait AssertEquals<Cb:'static, E:'static, T: PartialEq + std::fmt::Debug> {
    /// User defined function for extracting a collection data of a specific
    /// type from the proessor arguments
    fn extracted<'a>(&self, args: &'a ProcessorResult<Cb, E>) -> Option<&'a T>;

    /// The expected value to compare to the actual value to
    fn expected(&self) -> &T;
}

impl<Cb:'static, E:'static, T: PartialEq + std::fmt::Debug> std::fmt::Display for dyn AssertEquals<Cb, E, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", "assert_equals")
    }
}

impl<Cb:'static, E:'static, T> predicates::reflection::PredicateReflection for dyn AssertEquals<Cb, E, T> where
    T: PartialEq + std::fmt::Debug
{
}

impl<Cb:'static, E:'static, T> Predicate<ProcessorResult<Cb, E>> for dyn AssertEquals<Cb, E, T>
where
    T: PartialEq + std::fmt::Debug,
{
    fn eval(&self, args: &ProcessorResult<Cb, E>) -> bool {
        self.extracted(args)
            .map(|actual| actual == self.expected())
            .unwrap_or(false)
    }
}

/// Asserts some extracted data from ProcessorResult passes a predicate.  
pub trait Assert<Cb:'static, E:'static, T> {
    fn extracted<'a>(&self, args: &'a ProcessorResult<Cb, E>) -> Option<&'a T>;

    fn assert_inner(&self, args: &T) -> bool;
}

/// Asserts that the actual is equal to the given expected
#[derive(PartialEq, Debug)]
pub struct CallbackDataEquals<Cb, E>(pub Cb, std::marker::PhantomData<E>);

impl<Cb, E:'static> predicates::Predicate<ProcessorResult<Cb, E>> for CallbackDataEquals<Cb, E>
where
    Cb: PartialEq + std::fmt::Debug + 'static
{
    fn eval(&self, args: &ProcessorResult<Cb, E>) -> bool {
        self.extracted(args)
            .map(|actual| {
                actual == self.expected()
            })
            .unwrap_or(false)
    }
}

impl<Cb, E:'static> AssertEquals<Cb, E, Cb> for CallbackDataEquals<Cb, E> 
where
    Cb: PartialEq + std::fmt::Debug + 'static
{
    fn extracted<'a>(&self, args: &'a ProcessorResult<Cb, E>) -> Option<&'a Cb> {
        match &args.callback_data {
            Some(GhostCallbackData::Response(Ok(cb))) => Some(cb),
            _ => None
        } 
    }
    fn expected(&self) -> &Cb {
        &self.0
    }
}

impl<Cb, E> std::fmt::Display for CallbackDataEquals<Cb, E>
where
    Cb: PartialEq + std::fmt::Debug + 'static {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<Cb, E:'static> Processor<Cb, E> 
    for CallbackDataEquals<Cb, E> where Cb : std::fmt::Debug + 'static + PartialEq {
    fn test(&self, args: &ProcessorResult<Cb, E>) {
        let actual = self.extracted(args);
        assert_eq!(Some(self.expected()), actual);
    }

    fn name(&self) -> String {
        format!("{:?}", self.0).to_string()
    }
}

impl<Cb, E:'static> predicates::reflection::PredicateReflection for CallbackDataEquals<Cb, E> 
where Cb : std::fmt::Debug + 'static + PartialEq {}


/// Asserts using an arbitrary predicate over a lib3h server protocol event
pub struct CallbackDataAssert<Cb, E>(pub Box<dyn Predicate<Cb>>, std::marker::PhantomData<E>);

impl<Cb:'static, E:'static> Assert<Cb, E, Cb> for CallbackDataAssert<Cb, E> {
    fn extracted<'a>(&self, args: &'a ProcessorResult<Cb, E>) -> Option<&'a Cb> {
        match &args.callback_data {
            Some(GhostCallbackData::Response(Ok(cb))) => Some(cb),
            _ => None
        } 
    }

    fn assert_inner(&self, cb: &Cb) -> bool {
        self.0.eval(&cb)
    }
}


impl<Cb:'static, E:'static> Processor<Cb, E> for CallbackDataAssert<Cb, E> {
    fn test(&self, args: &ProcessorResult<Cb, E>) {
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

impl<Cb:'static, E:'static> Predicate<ProcessorResult<Cb, E>> for CallbackDataAssert<Cb, E> {
    fn eval(&self, args: &ProcessorResult<Cb, E>) -> bool {
        self.extracted(args)
            .map(|actual| self.assert_inner(&actual))
            .unwrap_or(false)
    }
}

impl<Cb, E:'static> std::fmt::Display for CallbackDataAssert<Cb, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", "callback data assertion")
    }
}

impl<Cb, E:'static> predicates::reflection::PredicateReflection for CallbackDataAssert<Cb, E> {}

/// Asserts work was done
#[derive(PartialEq, Debug)]
pub struct DidWorkAssert<Cb, E>(std::marker::PhantomData<Cb>, std::marker::PhantomData<E>);

impl<Cb:'static, E:'static> Processor<Cb, E> for DidWorkAssert<Cb, E> {
    fn test(&self, args: &ProcessorResult<Cb, E>) {
        assert!(args.did_work);
    }

    fn name(&self) -> String {
        format!("{:?}", "DidWorkAssert").to_string()
    }
}

impl<Cb:'static, E:'static> Predicate<ProcessorResult<Cb, E>> for DidWorkAssert<Cb, E> {
    fn eval(&self, args: &ProcessorResult<Cb, E>) -> bool {
        args.did_work
    }
}

impl<Cb:'static, E:'static> std::fmt::Display for DidWorkAssert<Cb, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?} did work", self.name())
    }
}

impl<Cb:'static, E:'static> predicates::reflection::PredicateReflection for DidWorkAssert<Cb, E> {}

#[allow(unused_macros)]
/// Convenience function that asserts only one particular equality predicate
/// passes for a collection of . See assert_processed for
macro_rules! assert_callback_eq {
    ($ghost_can_track:ident, //: &mumut t Vec<&mut Box<dyn Networkghost_actor>>,
     $user_data:ident,
     $equal_to:ident,// Box<dyn Processor>,
     $e_type:tt
    ) => {{
        let p = Box::new($crate::ghost_test_harness::CallbackDataEqual($equal_to.clone()));
        assert_callback_processed!($ghost_can_track, $user_data, $equal_to.clone(), $e_type, p)
    }};
}

#[allow(unused_macros)]
macro_rules! process_one {
    ($ghost_can_track: ident,
     $user_data: ident,
     $previous: ident,
     $callback_data: ident,
     $errors: ident
    ) => {{
        let did_work = $ghost_can_track
            .process(&mut $user_data)
            .map_err(|err| dbg!(err))
            .map(|did_work| did_work.into())
            .unwrap_or(false);
        if !did_work {
        } else {
            let processor_result = $crate::ghost_test_harness::ProcessorResult<_, _, _> {
                did_work,
                callback_data : $callback_data,
                user_data : $user_data,
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
     $e_type:tt,
     $processor:ident
 ) => {
     {
        let mut previous = Vec::new();
        let mut errors: Vec<(
            Box<dyn $crate::ghost_test_harness::Processor<_, $e_type>>,
            Option<$crate::ghost_test_harness::ProcessorResult<_, $e_type>>,
        )> = Vec::new();

       let mut callback_data = None;
       let cb: 
           $crate::ghost_actor::GhostCallback<_, _, _, _> =
           Box::new(|_user_data, _context, callback_data| {
               callback_data = Some(callback_data);
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

            process_one!($ghost_can_track, $user_data, 
                previous, callback_data, errors);
            if errors.is_empty() {
                break;
            }
        }

           for (p, args) in errors {
               if let Some(args) = args {
                   p.test(&args)
               } else {
                   // Make degenerate result which should fail
                   p.test(&$crate::ghost_test_harness::ProcessorResult {
                       previous: vec![],
                       callback_data: None,
                       did_work: false,
                       user_data : $user_data
                   })
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

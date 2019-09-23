/// A test harness for network engines. Provides specialized assertion functions
/// to verify predicates have passed, calling the engine process function as many
/// times a necessary until success (up to a hard coded number of iterations, currently).
use predicates::prelude::*;

use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol, uri::Lib3hUri};

use crate::utils::seeded_prng::SeededBooleanPrng;

use std::sync::Mutex;

#[allow(dead_code)]
pub const MAX_PROCESSING_LOOPS: usize = 100;

lazy_static! {

    pub static ref BOOLEAN_PRNG: Mutex<SeededBooleanPrng> = {

        // generate a random seed here
        // if you see an error "sometimes" manually paste the seed from the logs in here and
        // hardcode it for debugging
        // e.g. something like
        let seed = [1, 2];
        // let seed = [12290055440097485507, 11402434335878553749];
        // let seed = [rand::random::<u64>(), rand::random::<u64>()];
        // let seed = [1840432774656682167, 15353179927896983378];

        println!("seed is: {:?}", &seed);
        let seeded_boolean_prng = SeededBooleanPrng::from(seed);

        Mutex::new(seeded_boolean_prng)

    };

}

/// Represents all useful state after a single call to an engine's process function
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ProcessorResult {
    /// Whether the engine denoted by engine_name reported doing work or not
    pub did_work: bool,
    /// The name of the engine which produced these results
    pub engine_name: String,
    /// All events produced by the last call to process for engine by denoted by engine_name
    pub events: Vec<Lib3hServerProtocol>,
    /// All previously processed results, regardless of engine name
    pub previous: Vec<ProcessorResult>,
}

/// An assertion style processor which provides a
/// predicate over ProcessorResult (the eval function) and a
/// test function which will break control flow similar to
/// how calling assert! or assert_eq! would.
pub trait Processor: Predicate<ProcessorResult> {
    /// Processor name, for debugging and mapping purposes
    fn name(&self) -> String {
        "default_processor".into()
    }

    /// Test the predicate function. Should interrupt control
    /// flow with a useful error if self.eval(args) is false.
    fn test(&self, args: &ProcessorResult);
}

/// Asserts some extracted data from ProcessorResult is equal to an expected instance.
pub trait AssertEquals<T: PartialEq + std::fmt::Debug> {
    /// User defined function for extracting a collection data of a specific
    /// type from the proessor arguments
    fn extracted(&self, args: &ProcessorResult) -> Vec<T>;

    /// The expected value to compare to the actual value to
    fn expected(&self) -> T;
}

impl<T: PartialEq + std::fmt::Debug> std::fmt::Display for dyn AssertEquals<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", "assert_equals")
    }
}
impl<T> predicates::reflection::PredicateReflection for dyn AssertEquals<T> where
    T: PartialEq + std::fmt::Debug
{
}

impl<T> Predicate<ProcessorResult> for dyn AssertEquals<T>
where
    T: PartialEq + std::fmt::Debug,
{
    fn eval(&self, args: &ProcessorResult) -> bool {
        let extracted = self.extracted(args);
        extracted
            .iter()
            .find(|actual| **actual == self.expected())
            .is_some()
    }
}

/// Asserts some extracted data from ProcessorResult passes a predicate.
pub trait Assert<T> {
    fn extracted(&self, args: &ProcessorResult) -> Vec<T>;

    fn assert_inner(&self, args: &T) -> bool;
}

/// Asserts that the actual is equal to the given expected
#[allow(dead_code)]
#[derive(PartialEq, Debug)]
pub struct Lib3hServerProtocolEquals(pub Lib3hServerProtocol);

/// Asserts using an arbitrary predicate over a lib3h server protocol event
#[allow(dead_code)]
pub struct Lib3hServerProtocolAssert(pub Box<dyn Predicate<Lib3hServerProtocol>>);

/// Asserts work was done
#[allow(dead_code)]
#[derive(PartialEq, Debug)]
pub struct DidWorkAssert(pub String /* engine name */);

impl Processor for Lib3hServerProtocolAssert {
    fn test(&self, args: &ProcessorResult) {
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
        "Lib3hServerProtocolAssert".to_string()
    }
}

impl Processor for DidWorkAssert {
    fn test(&self, args: &ProcessorResult) {
        assert!(args.engine_name == self.0);
        assert!(args.did_work);
    }

    fn name(&self) -> String {
        format!("{:?}", self).to_string()
    }
}

impl Predicate<ProcessorResult> for DidWorkAssert {
    fn eval(&self, args: &ProcessorResult) -> bool {
        args.engine_name == self.0 && args.did_work
    }
}

impl Assert<Lib3hServerProtocol> for Lib3hServerProtocolAssert {
    fn extracted(&self, args: &ProcessorResult) -> Vec<Lib3hServerProtocol> {
        args.events.iter().map(|x| x.clone()).collect()
    }

    fn assert_inner(&self, x: &Lib3hServerProtocol) -> bool {
        self.0.eval(&x)
    }
}

impl predicates::Predicate<ProcessorResult> for Lib3hServerProtocolEquals {
    fn eval(&self, args: &ProcessorResult) -> bool {
        self.extracted(args)
            .iter()
            .find(|actual| **actual == self.expected())
            .is_some()
    }
}

impl Predicate<ProcessorResult> for Lib3hServerProtocolAssert {
    fn eval(&self, args: &ProcessorResult) -> bool {
        let extracted = self.extracted(args);
        extracted
            .iter()
            .find(|actual| self.assert_inner(*actual))
            .is_some()
    }
}

impl Processor for Lib3hServerProtocolEquals {
    fn test(&self, args: &ProcessorResult) {
        let extracted = self.extracted(args);
        let actual = extracted.iter().find(|actual| **actual == self.expected());
        assert_eq!(Some(&self.expected()), actual.or(extracted.first()));
    }

    fn name(&self) -> String {
        format!("{:?}", self).to_string()
    }
}

impl AssertEquals<Lib3hServerProtocol> for Lib3hServerProtocolEquals {
    fn extracted(&self, args: &ProcessorResult) -> Vec<Lib3hServerProtocol> {
        args.events.iter().map(|x| x.clone()).collect()
    }
    fn expected(&self) -> Lib3hServerProtocol {
        self.0.clone()
    }
}

impl std::fmt::Display for Lib3hServerProtocolEquals {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::fmt::Display for Lib3hServerProtocolAssert {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", "Lib3hServer protocol assertion")
    }
}

impl std::fmt::Display for DidWorkAssert {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?} did work", self.0)
    }
}

impl predicates::reflection::PredicateReflection for Lib3hServerProtocolEquals {}
impl predicates::reflection::PredicateReflection for Lib3hServerProtocolAssert {}
impl predicates::reflection::PredicateReflection for DidWorkAssert {}

#[allow(unused_macros)]
/// Convenience function that asserts only one particular equality predicate
/// passes for a collection of engines. See assert_processed for
/// more information. equal_to is compared to the actual and aborts if not actual.
macro_rules! assert_processed_eq {
    ($engine1:ident, //: &mumut t Vec<&mut Box<dyn NetworkEngine>>,
     $engine2:ident, //: &mumut t Vec<&mut Box<dyn NetworkEngine>>,
     $equal_to:ident,// Box<dyn Processor>,
    ) => {{
        let p = Box::new(Lib3hServerProtocolEquals($equal_to));
        assert_one_processed!($engine1, $engine2, p)
    }};
}

#[allow(unused_macros)]
/// Convenience function that asserts only one particular predicate
/// passes for a collection of engines. See assert_processed for
/// more information.
macro_rules! assert_one_processed {
    ($engine1:ident,
     $engine2:ident,
     $processor:ident,
    $should_abort:expr
    ) => {{
        let processors = vec![$processor];
        let result = assert_processed!($engine1, $engine2, processors, $should_abort);
        result
    }};
    ($engine1:ident,
     $engine2:ident,
     $processor:ident
     ) => {
        assert_one_processed!($engine1, $engine2, $processor, true)
    };
}

#[allow(unused_macros)]
macro_rules! process_one {
    ($engine: ident,
  $previous: ident,
  $errors: ident
  ) => {{
        let (did_work, events) = $engine
            .process()
            .map_err(|err| dbg!(err))
            .unwrap_or((false, vec![]));
        if events.is_empty() {
        } else {
            let events = dbg!(events);
            let processor_result = $crate::utils::processor_harness::ProcessorResult {
                did_work,
                events,
                engine_name: $engine.name(),
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
            if !processor_result.events.is_empty() {
                $previous.push(processor_result.clone());
            }
        }
    }};
}

/// Asserts that a collection of engines produce events
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
    ($engine1:ident,
  $engine2:ident,
  $processors:ident
 ) => {
        assert_processed!($engine1, $engine2, $processors, true)
    };
    ($engine1:ident,
     $engine2:ident,
     $processors:ident,
     $should_abort:expr) => {{
        let mut previous = Vec::new();
        let mut errors: Vec<(
            Box<dyn $crate::utils::processor_harness::Processor>,
            Option<$crate::utils::processor_harness::ProcessorResult>,
        )> = Vec::new();

        for p in $processors {
            errors.push((p, None))
        }

        // each epoc represents one "random" engine processing once
        for epoc in 0..$crate::utils::processor_harness::MAX_PROCESSING_LOOPS {
            let b = $crate::utils::processor_harness::BOOLEAN_PRNG
                .lock()
                .expect("could not acquire lock on boolean prng")
                .next()
                .expect("could not generate a new seeded prng value");
            println!(
                "seed: {:?}, epoc: {:?}, prng: {:?}, previous: {:?}",
                $crate::utils::processor_harness::BOOLEAN_PRNG
                    .lock()
                    .expect("could not acquire lock on boolean prng")
                    .seed,
                epoc,
                b,
                previous
            );

            // pick either engine1 or engine2 with equal probability
            if b {
                process_one!($engine1, previous, errors);
            } else {
                process_one!($engine2, previous, errors);
            }
            if errors.is_empty() {
                break;
            }
        }

        if $should_abort {
            for (p, args) in errors {
                if let Some(args) = args {
                    p.test(&args)
                } else {
                    // Make degenerate result which should fail
                    p.test(&$crate::utils::processor_harness::ProcessorResult {
                        engine_name: "none".into(),
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
/// Creates a processor that verifies a connected data response is produced
/// by an engine
#[allow(dead_code)]
pub fn is_connected(request_id: &str, uri: Lib3hUri) -> Lib3hServerProtocolEquals {
    Lib3hServerProtocolEquals(Lib3hServerProtocol::Connected(ConnectedData {
        request_id: request_id.into(),
        uri,
    }))
}

#[allow(unused_macros)]
macro_rules! wait_connect {
    (
        $me:ident,
        $connect_data: ident,
        $other: ident
    ) => {{
        let _connect_data = $connect_data;
        let re = regex::Regex::new("ConnectedData").expect("valid regex");
        let assertion = Box::new(predicates::prelude::predicate::function(move |x| {
            let to_match = format!("{:?}", x);
            re.is_match(&to_match)
        }));

        let predicate: Box<dyn $crate::utils::processor_harness::Processor> = Box::new(
            $crate::utils::processor_harness::Lib3hServerProtocolAssert(assertion),
        );

        let result = assert_one_processed!($me, $other, predicate);
        result
    }};
}

/// Waits for work to be done. Will interrupt the program if no work was done and should_abort
/// is true
#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_engine_wrapper_did_work {
    ($engine: ident,
     $should_abort: expr
    ) => {{
        let timeout = std::time::Duration::from_millis(2000);
        $crate::wait_engine_wrapper_did_work!($engine, $should_abort, timeout)
    }};
    ($engine:ident) => {
        $crate::wait_engine_wrapper_did_work!($engine, true)
    };
    ($engine: ident,
     $should_abort: expr,
     $timeout : expr
      ) => {{
        let mut did_work = false;
        let clock = std::time::SystemTime::now();

        for i in 0..20 {
            let (did_work_now, _) = $engine
                .process()
                .map_err(|e| println!("ghost actor processing error: {:?}", e))
                .unwrap_or((false, vec![]));
            did_work = did_work_now;
            if did_work {
                break;
            }
            let elapsed = clock.elapsed().unwrap();
            if elapsed > $timeout {
                break;
            }
            println!("[{}] wait_engine_wrapper_did_work", i);
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
    ($engine: ident) => {{
        let mut did_work;
        loop {
            did_work = $crate::wait_engine_wrapper_did_work!($engine, false);
            if !did_work {
                break;
            }
        }
        did_work
    }};
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_until_did_work {
    ($engine: ident) => {{
        let mut did_work;
        loop {
            did_work = $crate::wait_engine_wrapper_did_work!($engine, false);
            if did_work {
                break;
            }
        }
        did_work
    }};
}

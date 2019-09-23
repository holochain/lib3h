/// A test harness for network engines. Provides specialized assertion functions
/// to verify predicates have passed, calling the engine process function as many
/// times a necessary until success (up to a hard coded number of iterations, currently).
use predicates::prelude::*;

use lib3h_protocol::protocol_server::Lib3hServerProtocol;

use crate::utils::seeded_prng::SeededBooleanPrng;

use std::sync::Mutex;

#[allow(dead_code)]
pub const MAX_PROCESSING_LOOPS: usize = 100;

#[allow(dead_code)]
pub const MAX_DID_WEIGHT_LOOPS: usize = 20;

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

    /// Test the predicate function. Should interrupt control
    /// flow with a useful error if self.eval(args) is false.
    fn test(&self, args: &ProcessorResult);
}

/// Asserts some extracted data from ProcessorResult is equal to an expected instance.
pub trait AssertEquals<T: PartialEq + std::fmt::Debug> {
    /// User defined function for extracting a collection data of a specific
    /// type from the processor arguments
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

impl<T> Processor for dyn AssertEquals<T>
where
    T: PartialEq + std::fmt::Debug,
{
    fn test(&self, args: &ProcessorResult) -> () {
        let extracted = self.extracted(args);
        let actual = extracted.iter().find(|actual| **actual == self.expected());
        assert_eq!(Some(&self.expected()), actual.or(extracted.first()));
    }
}


/// Asserts some extracted data from ProcessorResult matches a regular expression
/// Will invoke `assert_eq!(regex, format!("{:?}", actual))` upon failure for easy
/// to compare output
pub trait AssertRegex<T: std::fmt::Debug> {
    /// User defined function for extracting a collection data of a specific
    /// type from the processor arguments
    fn extracted(&self, args: &ProcessorResult) -> Vec<T>;

    /// The regex value to match against the actual value
    fn expected(&self) -> regex::Regex;
}

impl<T> Predicate<ProcessorResult> for dyn AssertRegex<T>
where
    T: std::fmt::Debug,
{
    fn eval(&self, args: &ProcessorResult) -> bool {
        let extracted = self.extracted(args);
        extracted
            .iter()
            .find(|actual| self.expected().is_match(format!("{:?}", **actual).as_str()))
            .is_some()
    }
}

impl<T> Processor for dyn AssertRegex<T>
where
    T: std::fmt::Debug,
{
    fn test(&self, args: &ProcessorResult) -> () {
        if !self.eval(args) {
            let actual = self.extracted(args).first().map(
                |a| format!("{:?}", a)).unwrap_or("None".to_string());
             assert_eq!(
                self.expected().as_str(),
                actual.as_str()
             )
        }
   }
}


impl<T: std::fmt::Debug> std::fmt::Display for dyn AssertRegex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", "assert_regex")
    }
}

impl<T> predicates::reflection::PredicateReflection for dyn AssertRegex<T> where
    T: std::fmt::Debug
{
}

/// Asserts some extracted data from ProcessorResult passes a predicate.
pub trait Assert<T> {
    fn extracted(&self, args: &ProcessorResult) -> Vec<T>;

    fn assert_inner(&self, args: &T) -> bool;
}

impl<T> Predicate<ProcessorResult> for dyn Assert<T>
{
    fn eval(&self, args: &ProcessorResult) -> bool {
        let extracted = self.extracted(args);
        extracted
            .iter()
            .find(|actual| self.assert_inner(*actual))
            .is_some()
    }
}

impl<T> Processor for dyn Assert<T>
{
    fn test(&self, args: &ProcessorResult) -> () {
        assert!(self.eval(args))
    }
}

impl<T> predicates::reflection::PredicateReflection for dyn Assert<T>
{
}

impl<T> std::fmt::Display for dyn Assert<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", "assert_processed")
    }
}


/// Asserts that the actual is equal to the given expected
#[allow(dead_code)]
#[derive(PartialEq, Debug)]
pub struct Lib3hServerProtocolEquals(pub Lib3hServerProtocol);

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

impl predicates::reflection::PredicateReflection for Lib3hServerProtocolEquals {}

/// Asserts that the actual matches the given regular expression
#[allow(dead_code)]
#[derive(Debug)]
pub struct Lib3hServerProtocolRegex(regex::Regex);

impl AssertRegex<Lib3hServerProtocol> for Lib3hServerProtocolRegex {
    fn extracted(&self, args: &ProcessorResult) -> Vec<Lib3hServerProtocol> {
        args.events.iter().map(|x| x.clone()).collect()
    }

    fn expected(&self) -> regex::Regex {
        self.0.clone()
    }
}

impl std::fmt::Display for Lib3hServerProtocolRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl predicates::reflection::PredicateReflection for Lib3hServerProtocolRegex {}

/// Asserts using an arbitrary predicate over a lib3h server protocol event
#[allow(dead_code)]
pub struct Lib3hServerProtocolAssert(pub Box<dyn Predicate<Lib3hServerProtocol>>);

impl Assert<Lib3hServerProtocol> for Lib3hServerProtocolAssert {
    fn extracted(&self, args: &ProcessorResult) -> Vec<Lib3hServerProtocol> {
        args.events.iter().map(|x| x.clone()).collect()
    }

    fn assert_inner(&self, x: &Lib3hServerProtocol) -> bool {
        self.0.eval(&x)
    }
}

impl std::fmt::Display for Lib3hServerProtocolAssert {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", "Lib3hServer protocol assertion")
    }
}

impl predicates::reflection::PredicateReflection for Lib3hServerProtocolAssert {}

/// Asserts work was done
#[allow(dead_code)]
#[derive(PartialEq, Debug)]
pub struct DidWorkAssert(pub String /* engine name */);
impl Processor for DidWorkAssert {
    fn test(&self, args: &ProcessorResult) {
        assert!(args.engine_name == self.0);
        assert!(args.did_work);
    }
}

impl Predicate<ProcessorResult> for DidWorkAssert {
    fn eval(&self, args: &ProcessorResult) -> bool {
        args.engine_name == self.0 && args.did_work
    }
}

impl std::fmt::Display for DidWorkAssert {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?} did work", self.0)
    }
}

impl predicates::reflection::PredicateReflection for DidWorkAssert {}

#[allow(unused_macros)]
/// Convenience function that asserts only one particular equality predicate
/// (over a lib3h server protocol message)
/// passes for a collection of engines. See assert_processed for
/// more information. equal_to is compared to the actual and aborts if not actual.
macro_rules! assert2_msg_eq {
    ($engine1:ident,
     $engine2:ident,
     $equal_to:expr
    ) => {{
        let p = Box::new($crate::utils::processor_harness::Lib3hServerProtocolEquals($equal_to));
        assert2_processed!($engine1, $engine2, p)
    }};
}

#[allow(unused_macros)]
#[macro_export]
/// Convenience function that asserts only one particular equality predicate
/// (over a lib3h server protocol message)
/// passes for two engine wrappers. See assert_processed for
/// more information. regex is matches against the actual value and aborts if not present
macro_rules! assert2_msg_matches {
    ($engine1:ident,
     $engine2:ident,
     $regex:expr
    ) => {{
        let p = Box::new($crate::utils::processor_harness::Lib3hServerProtocolRegex(regex::Regex::new($regex)));
        assert2_processed!($engine1, $engine2, p)
    }};
}

#[allow(unused_macros)]
#[macro_export]
/// Convenience function that asserts only one particular equality predicate
/// (over a lib3h server protocol message)
/// passes for two engine wrappers. See assert_processed for
/// more information. regex is matches against the actual value and aborts if not present
macro_rules! assert_msg_matches {
    ($engine:ident,
     $regex:expr
    ) => {
        $crate::utils::processor_harness::assert2_msg_matches($engine, $engine, $regex)
    };
}


#[allow(unused_macros)]
#[macro_export]
/// Convenience function that asserts only one particular equality predicate
/// (over a lib3h server protocol message)
/// passes for two engine wrappers. See assert_processed for
/// more information. regex is matches against the actual value and aborts if not present
macro_rules! assert2_msg_matches_all {
    ($engine1:ident,
     $engine2:ident,
     $regexes:expr
    ) => {{

        let processors = $regexes.into_iter().map(|re|
            Box::new($crate::utils::processor_harness::Lib3hServerProtocolRegex(
                    regex::Regex::new(re)
                        .expect(format!("Regex must be syntactically correct: {:?}", re).as_str())
                    )))
            .collect();
        $crate::utils::processor_harness::assert2_processed!($engine1, $engine2, processors)
    }};
}

#[allow(unused_macros)]
#[macro_export]
/// Convenience function that asserts only one particular equality predicate
/// (over a lib3h server protocol message)
/// passes for two engine wrappers. See assert_processed for
/// more information. regex is matches against the actual value and aborts if not present
macro_rules! assert_msg_matches_all {
    ($engine:ident,
     $regexes:expr
    ) => {
        $crate:utils::processor_harness::assert2_msg_matches_all!($engine, $engine, $regexes)
    };
}


/// Internal function to process one engine of a possibly
/// multiple engine scenario
#[allow(unused_macros)]
#[macro_export]
macro_rules! process_one_engine {
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
///
/// This is a public function but most likely won't be used in preferred
/// over a specialized form for specific processor implementations.
#[allow(unused_macros)]
#[macro_export]
macro_rules! assert2_processed_all {
    ($engine1:ident,
     $engine2:ident,
     $processors:expr
    ) => {{
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
                $crate::process_one_engine!($engine1, previous, errors);
            } else {
                $crate::process_one_engine!($engine2, previous, errors);
            }
            if errors.is_empty() {
                break;
            }
        }

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
        previous
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
#[macro_export]
macro_rules! assert2_processed {
    ($engine1:ident,
     $engine2:ident,
     $processor:expr
    ) =>
    {{
         let processors = vec![$processor];
         $crate::assert2_processed_all!(
             $engine1, $engine2, processors)
    }}
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! assert_processed_all {
    ($engine:ident,
     $processors:expr
    ) => {
        // HACK make a singular version
        $crate::assert2_processed_all!(
            $engine, $engine, $processors)
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! assert_processed {
    ($engine:ident,
     $processor:expr
    ) => {
        // HACK make a singular version
        $crate::assert2_processed!(
            $engine, $engine, $processor)
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_connect {
    (
        $me:ident,
        $connect_data: expr,
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

        let result = assert2_processed!($me, $other, predicate);
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

        for epoc in 0..$crate::utils::processor_harness::MAX_PROCESSING_LOOPS {
            let (did_work_now, results) = $engine
                .process()
                .map_err(|e| error!("ghost actor processing error: {:?}", e))
                .unwrap_or((false, vec![]));
            did_work = did_work_now;
            if did_work {
                break;
            }
            let elapsed = clock.elapsed().unwrap();
            if elapsed > $timeout {
                break;
            }
            trace!("[{}] wait_engine_wrapper_did_work: {:?}", epoc, results);
            std::thread::sleep(std::time::Duration::from_millis(1))
        }
        if $should_abort {
            assert!(did_work);
        }
        did_work
    }};
}

/// Continues processing the engine wrapper until no more work is observed
#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_engine_wrapper_until_no_work {
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

/// A test harness for network engines. Provides specialized assertion functions
/// to verify predicates have passed, calling the engine process function as many
/// times a necessary until success (up to a hard coded number of iterations, currently).
use predicates::prelude::*;

use lib3h_protocol::protocol_server::Lib3hServerProtocol;

use crate::utils::seeded_prng::SeededBooleanPrng;

use std::sync::Mutex;

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

        trace!("seed is: {:?}", &seed);
        let seeded_boolean_prng = SeededBooleanPrng::from(seed);

        Mutex::new(seeded_boolean_prng)

    };

}

#[allow(dead_code)]
pub const DEFAULT_MAX_ITERS: u64 = 1000;
#[allow(dead_code)]
pub const DEFAULT_MAX_RETRIES: u64 = 5;
#[allow(dead_code)]
pub const DEFAULT_DELAY_INTERVAL_MS: u64 = 1;
#[allow(dead_code)]
pub const DEFAULT_TIMEOUT_MS: u64 = 1000;
#[allow(dead_code)]
pub const DEFAULT_SHOULD_ABORT: bool = true;
#[allow(dead_code)]
pub const DEFAULT_WAIT_DID_WORK_MAX_ITERS: u64 = 50;
#[allow(dead_code)]
pub const DEFAULT_WAIT_DID_WORK_TIMEOUT_MS: u64 = 50;

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
    #[allow(dead_code)]
    pub fn wait_did_work_defaults() -> Self {
        Self {
            max_iters: DEFAULT_WAIT_DID_WORK_MAX_ITERS,
            timeout_ms: DEFAULT_WAIT_DID_WORK_TIMEOUT_MS,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
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

    fn prefer<'a>(&self, result1:&'a ProcessorResult, _result2:&'a ProcessorResult) -> &'a ProcessorResult {
        result1
    }
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
            let actual = self
                .extracted(args)
                .first()
                .map(|a| format!("{:?}", a))
                .unwrap_or("None".to_string());
            assert_eq!(self.expected().as_str(), actual.as_str())
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Display for dyn AssertRegex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", "assert_regex")
    }
}

impl<T> predicates::reflection::PredicateReflection for dyn AssertRegex<T> where T: std::fmt::Debug {}

/// Asserts some extracted data from ProcessorResult passes a predicate.
pub trait Assert<T> {
    fn extracted(&self, args: &ProcessorResult) -> Vec<T>;

    fn assert_inner(&self, args: &T) -> bool;
}

impl<T> Predicate<ProcessorResult> for dyn Assert<T> {
    fn eval(&self, args: &ProcessorResult) -> bool {
        let extracted = self.extracted(args);
        extracted
            .iter()
            .find(|actual| self.assert_inner(*actual))
            .is_some()
    }
}

impl<T> Processor for dyn Assert<T> {
    fn test(&self, args: &ProcessorResult) -> () {
        assert!(self.eval(args))
    }
}

impl<T> predicates::reflection::PredicateReflection for dyn Assert<T> {}

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

impl Predicate<ProcessorResult> for Lib3hServerProtocolEquals {
    fn eval(&self, args: &ProcessorResult) -> bool {
        let extracted = self.extracted(args);
        extracted
            .iter()
            .find(|actual| **actual == self.expected())
            .is_some()
    }
}

impl Processor for Lib3hServerProtocolEquals {
    fn test(&self, args: &ProcessorResult) {
        let extracted = self.extracted(args);
        let actual = extracted.iter().find(|actual| **actual == self.expected());
        assert_eq!(Some(&self.expected()), actual.or(extracted.first()));
    }
}

/// Asserts that the actual matches the given regular expression
#[allow(dead_code)]
#[derive(Debug)]
pub struct Lib3hServerProtocolRegex(pub regex::Regex);

impl AssertRegex<Lib3hServerProtocol> for Lib3hServerProtocolRegex {
    fn extracted(&self, args: &ProcessorResult) -> Vec<Lib3hServerProtocol> {
        args.events.iter().map(|x| x.clone()).collect()
    }

    fn expected(&self) -> regex::Regex {
        self.0.clone()
    }
}

impl Predicate<ProcessorResult> for Lib3hServerProtocolRegex {
    fn eval(&self, args: &ProcessorResult) -> bool {
        let extracted = self.extracted(args);
        extracted
            .iter()
            .find(|actual| self.expected().is_match(format!("{:?}", **actual).as_str()))
            .is_some()
    }
}

impl Processor for Lib3hServerProtocolRegex {
    fn test(&self, args: &ProcessorResult) -> () {
        if !self.eval(args) {
            let actual = self
                .extracted(args)
                .first()
                .map(|a| format!("{:?}", a))
                .unwrap_or("None".to_string());
            assert_eq!(self.expected().as_str(), actual.as_str())
        }
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

impl Predicate<ProcessorResult> for Lib3hServerProtocolAssert {
    fn eval(&self, args: &ProcessorResult) -> bool {
        let extracted = self.extracted(args);
        extracted
            .iter()
            .find(|actual| self.assert_inner(*actual))
            .is_some()
    }
}

impl Processor for Lib3hServerProtocolAssert {
    fn test(&self, args: &ProcessorResult) -> () {
        assert!(self.eval(args))
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
pub struct DidWorkAssert {
    pub engine_name: String,
    pub should_assert: bool,
}

impl Processor for DidWorkAssert {
    fn test(&self, args: &ProcessorResult) {
        if self.should_assert {
            assert!(args.engine_name == self.engine_name);
            assert!(args.did_work);
        }
    }
}

impl Predicate<ProcessorResult> for DidWorkAssert {
    fn eval(&self, args: &ProcessorResult) -> bool {
        args.engine_name == self.engine_name && args.did_work
    }
}

impl std::fmt::Display for DidWorkAssert {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:?} did work (should_assert={:?})",
            self.engine_name, self.should_assert
        )
    }
}

impl predicates::reflection::PredicateReflection for DidWorkAssert {}

#[allow(unused_macros)]
/// Convenience function that asserts only one particular equality predicate
/// (over a lib3h server protocol message)
/// passes for two engine wrappers. See `assert2_processed` for more information.
/// `equal_to` is compared to the actual and aborts if not actual by default.
macro_rules! assert2_msg_eq {
    ($engine1:ident,
     $engine2:ident,
     $equal_to:expr
    ) => {{
        let options = $crate::utils::processor_harness::ProcessingOptions::default();
        assert2_msg_eq!($engine1, $engine2, $equal_to, options)
    }};
    ($engine1:ident,
     $engine2:ident,
     $equal_to:expr,
     $options:expr
    ) => {{
        let p = Box::new($crate::utils::processor_harness::Lib3hServerProtocolEquals(
            $equal_to,
        ));
        assert2_processed!($engine1, $engine2, p, $options)
    }};
}

#[allow(unused_macros)]
#[macro_export]
/// Convenience function that asserts only one particular equality predicate
/// (over a lib3h server protocol message)
/// passes for two engine wrappers. See `assert_processed` for
/// more information. `regex` is matched against the actual value and aborts if not present by
/// default
macro_rules! assert2_msg_matches {
    ($engine1: ident,
     $engine2: ident,
     $regex: expr
    ) => {{
        let options = $crate::utils::processor_harness::ProcessingOptions::default();
        $crate::assert2_msg_matches!($engine1, $engine2, $regex, options)
    }};
    ($engine1: ident,
     $engine2: ident,
     $regex: expr,
     $options: expr
    ) => {{
        let p = Box::new($crate::utils::processor_harness::Lib3hServerProtocolRegex(
            regex::Regex::new($regex)
                .expect(format!("[assert2_msg_matches] Invalid regex: {:?}", $regex).as_str()),
        ));
        $crate::assert2_processed!($engine1, $engine2, p, $options)
    }};
}

#[allow(unused_macros)]
#[macro_export]
/// Convenience function that asserts only one particular msg matches
/// a regular expression over a lib3h server protocol message.
/// This is a simplified version of `assert2_msg_matches` for one engine only.
macro_rules! assert_msg_matches {
    ($engine: ident,
     $regex: expr,
     $options: expr
    ) => {
        // TODO Hack make a single engine version
        $crate::assert2_msg_matches!($engine, $engine, $regex, $options)
    };
    ($engine: ident,
     $regex: expr
    ) => {
        // TODO Hack make a single engine version
        $crate::assert2_msg_matches!($engine, $engine, $regex)
    };
}

#[allow(unused_macros)]
#[macro_export]
/// Convenience function that asserts all regular expressions match
/// over a set of lib3h server protocol messages for two engine wrappers.
macro_rules! assert2_msg_matches_all {
    ($engine1:ident,
     $engine2:ident,
     $regexes:expr
    ) => {{
        let processors = $regexes
            .into_iter()
            .map(|re| {
                Box::new($crate::utils::processor_harness::Lib3hServerProtocolRegex(
                    regex::Regex::new(re)
                        .expect(format!("[assert2_msg_matches_all] Regex must be syntactically correct: {:?}", re).as_str()),
                ))
            })
            .collect();
        $crate::assert2_processed!($engine1, $engine2, processors)
    }};
}

#[allow(unused_macros)]
#[macro_export]
/// Convenience function that asserts all regular expressions match
/// over a set of lib3h server protocol messages for one engine wrapper.
macro_rules! assert_msg_matches_all {
    ($engine: ident,
     $regexes: expr
    ) => {
        $crate: utils::processor_harness::assert2_msg_matches_all!($engine, $engine, $regexes)
    };
    ($engine: ident,
     $regexes: expr,
     $options: expr
    ) => {
        $crate:
            utils::processor_harness::assert2_msg_matches_all!($engine, $engine, $regexes, $options)
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
            .map_err(|err| error!("[process_one_engine] process generated an error: {:?}", err))
            .unwrap_or((false, vec![]));
        if events.is_empty() {
        } else {
            trace!("[process_one_engine] by {} {:?}", $engine.name(), events);
            let processor_result = $crate::utils::processor_harness::ProcessorResult {
                did_work,
                events,
                engine_name: $engine.name(),
                previous: $previous.clone(),
            };
            let mut failed = Vec::new();

            for (processor, orig_processor_result) in $errors.drain(..) {
                let result = processor.eval(&processor_result.clone());
                if result {
                    // Simulate the succesful assertion behavior
                    processor.test(&processor_result.clone());
                // processor passed!
                } else {
                    // Cache the assertion error and trigger it later if we never
                    // end up passing
                    let orig_processor_result = orig_processor_result.unwrap_or_else(|| processor_result.clone());
                    let prefered_failed_result = processor.prefer(&orig_processor_result, &processor_result);
                    failed.push((processor, Some(prefered_failed_result.to_owned())));
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
     $processors:expr,
     $options: expr
    ) => {{
        let mut previous = Vec::new();
        let mut errors: Vec<(
            Box<dyn $crate::utils::processor_harness::Processor>,
            Option<$crate::utils::processor_harness::ProcessorResult>,
        )> = Vec::new();

        for p in $processors {
            errors.push((p, None))
        }

        let clock = std::time::SystemTime::now();
        let timeout = std::time::Duration::from_millis($options.timeout_ms);
        let delay_interval = std::time::Duration::from_millis($options.delay_interval_ms);
        // each epoc represents one "random" engine processing once
        for epoc in 0..$options.max_iters {
            let b = $crate::utils::processor_harness::BOOLEAN_PRNG
                .lock()
                .expect("could not acquire lock on boolean prng")
                .next()
                .expect("could not generate a new seeded prng value");
            trace!(
                "[processor_harness] seed: {:?}, epoc: {:?}, prng: {:?}, previous: {:?}",
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
            let elapsed = clock.elapsed().unwrap();
            if elapsed > timeout {
                trace!(
                    "[process_harness] epoc:{:?} timed out, elapsed {:?} ",
                    epoc,
                    elapsed.as_millis()
                );
                break;
            }
            std::thread::sleep(delay_interval)
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

/// Asserts that two engines produce events
/// matching just one predicate function. For the program
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
    ($engine1: ident,
     $engine2: ident,
     $processor: expr,
     $options: expr
    ) => {{
        let processors = vec![$processor];
        $crate::assert2_processed_all!($engine1, $engine2, processors, $options)
    }};
    ($engine1: ident,
     $engine2: ident,
     $processor: expr
    ) => {{
        let options = $crate::utils::processor_harness::ProcessingOptions::default();
        $crate::assert2_processed!($engine1, $engine2, $processor, options)
    }};
}

/// Asserts that one engine produces events
/// matching a set of predicate functions. For the program
/// to continue executing all processors must pass.
///
/// Multiple calls to process() will be made as needed for
/// the passed in processors to pass. It will failure after
/// ``$options.max_iters` iterations regardless.
///
/// Returns all observed processor results for use by
/// subsequent tests.
#[allow(unused_macros)]
#[macro_export]
macro_rules! assert_processed_all {
    ($engine: ident,
     $processors: expr,
     $options: expr
    ) => {{
        // HACK make a singular version
        $crate::assert2_processed_all!($engine, $engine, $processors, $options)
    }};
    ($engine: ident,
     $processors: expr
    ) => {{
        let options = $crate::utils::processor_harness::ProcessingOptions::default();
        $crate::assert_processed_all!($engine, $processors, options)
    }};
}

/// Asserts that one engine produces events
/// matching just one predicate function. For the program
/// to continue executing all processors must pass.
///
/// Multiple calls to process() will be made as needed for
/// the passed in processors to pass. It will failure after
/// `$options.max_iters` iterations regardless.
///
/// Returns all observed processor results for use by
/// subsequent tests.
#[allow(unused_macros)]
#[macro_export]
macro_rules! assert_processed {
    ($engine: ident,
     $processor: expr,
     $options: expr
    ) => {
        // HACK make a singular version
        $crate::assert2_processed!($engine, $engine, $processor, $options)
    };
    ($engine: ident,
     $processor: expr
    ) => {{
        let options = $crate::utils::processor_harness::ProcessingOptions::default();
        $crate::assert_processed!($engine, $processor, options)
    }};
}

/// `wait_connect!(a, connect_data, b)` waits until engine w4rapper `a` connects
/// using `connect_data` to engine wrapper `b`.
#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_connect {
    (
        $me:ident,
        $connect_data: expr,
        $other: ident
    ) => {{
        let _connect_data = $connect_data;

        $crate::assert2_msg_matches!($me, $other,
            "Connected\\(ConnectedData \\{ request_id: \"client_to_lib3_response_.*\", uri: Lib3hUri\\(\"nodepubkey:HcM.*\"\\) \\}\\)")
    }};
}

/// Waits for work to be done. Will interrupt the program if no work was done and should_abort
/// is true
#[allow(unused_macros)]
#[macro_export]
macro_rules! wait_engine_wrapper_did_work {
    ($engine: ident) => {{
        let options = $crate::utils::processor_harness::ProcessingOptions::wait_did_work_defaults();
        $crate::wait_engine_wrapper_did_work!($engine, options)
    }};
    ($engine: ident,
     $options: expr
    ) => {{
        let mut did_work = false;
        let clock = std::time::SystemTime::now();
        let timeout = std::time::Duration::from_millis($options.timeout_ms);
        let delay_interval = std::time::Duration::from_millis($options.delay_interval_ms);

        for epoc in 0..$options.max_iters {
            let (did_work_now, results) = $engine
                .process()
                .map_err(|e| error!("engine wrapper processing error: {:?}", e))
                .unwrap_or((false, vec![]));
            did_work = did_work_now;
            if did_work {
                break;
            }
            let elapsed = clock.elapsed().unwrap();
            if elapsed > timeout {
                trace!(
                    "[{}] wait_engine_wrapper_did_work: timeout elapsed={:?}, results={:?}",
                    epoc,
                    elapsed.as_millis(),
                    results
                );
                break;
            }
            trace!("[{}] wait_engine_wrapper_did_work: {:?}", epoc, results);
            std::thread::sleep(delay_interval)
        }
        if $options.should_abort {
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
        let options = $crate::utils::processor_harness::ProcessingOptions::default();
        $crate::wait_engine_wrapper_until_no_work!($engine, options)
    }};
    ($engine: ident, $options: expr) => {{
        let mut did_work = false;
        let clock = std::time::SystemTime::now();
        let timeout = std::time::Duration::from_millis($options.timeout_ms);
        let delay_interval = std::time::Duration::from_millis($options.delay_interval_ms);

        let did_work_options = $crate::utils::processor_harness::ProcessingOptions {
            should_abort: false,
            ..$crate::utils::processor_harness::ProcessingOptions::wait_did_work_defaults()
        };

        for i in 0..$options.max_iters {
            did_work = $crate::wait_engine_wrapper_did_work!($engine, did_work_options);
            if !did_work {
                break;
            }
            let elapsed = clock.elapsed().unwrap();
            if elapsed > timeout {
                trace!(
                    "[{:?}] wait_engine_wrapper_until_no_work timeout elapsed = {:?}",
                    i,
                    elapsed.as_millis()
                );
                break;
            }
            std::thread::sleep(delay_interval)
        }
        did_work
    }};
}

/// Continues processing two engine wrappers until they both exhibit work.
#[allow(unused_macros)]
#[macro_export]
macro_rules! wait2_engine_wrapper_did_work {
    ($engine1: ident,
     $engine2: ident,
     $options: expr
    ) => {{
        let processors = vec![
            Box::new($crate::utils::processor_harness::DidWorkAssert {
                engine_name: $engine1.name(),
                should_assert: $options.should_abort,
            }),
            Box::new($crate::utils::processor_harness::DidWorkAssert {
                engine_name: $engine2.name(),
                should_assert: $options.should_abort,
            }),
        ];
        $crate::assert2_processed_all!($engine1, $engine2, processors, $options)
    }};
    ($engine1: ident,
     $engine2: ident
    ) => {{
        let options = $crate::utils::processor_harness::ProcessingOptions::wait_did_work_defaults();
        $crate::wait2_engine_wrapper_did_work!($engine1, $engine2, options)
    }};
}

/// Continues processing the engine wrapper until nso more work is observed
#[allow(unused_macros)]
#[macro_export]
macro_rules! wait2_engine_wrapper_until_no_work {
    ($engine1: ident, $engine2: ident) => {{
        let options = $crate::utils::processor_harness::ProcessingOptions::default();
        $crate::wait2_engine_wrapper_until_no_work!($engine1, $engine2, options)
    }};
    ($engine1: ident, $engine2: ident, $options: expr) => {{
        let mut did_work;
        let clock = SystemTime::now();
        let timeout = std::time::Duration::from_millis($options.timeout_ms);
        let delay_interval = std::time::Duration::from_millis($options.delay_interval_ms);
        for i in 0..$options.max_iters {
            let did_work_options = ProcessingOptions {
                should_abort: false,
                ..$crate::utils::processor_harness::ProcessingOptions::wait_did_work_defaults()
            };
            did_work = $crate::wait2_engine_wrapper_did_work!($engine1, $engine2, did_work_options)
                .iter()
                .find(|result| result.did_work)
                .is_some();
            if !did_work {
                break;
            }
            let elapsed = clock.elapsed().unwrap();
            if elapsed > timeout {
                trace!(
                    "wait2_engine_Wrapper_until_no_work: timed out (over {:?} ms)",
                    $options.timeout_ms
                );
                break;
            }
            std::thread::sleep(delay_interval)
        }
        did_work
    }};
}

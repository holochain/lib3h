/// A test harness for network engines. Provides specialized assertion functions
/// to verify predicates have passed, calling the engine process function as many
/// times a necessary until success (up to a hard coded number of iterations, currently).
use predicates::prelude::*;

use lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_server::Lib3hServerProtocol,
};

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
#[allow(dead_code)]
#[derive(PartialEq, Debug)]
pub struct Lib3hServerProtocolEquals(pub Lib3hServerProtocol);

#[allow(dead_code)]
pub struct Lib3hServerProtocolAssert(pub Box<dyn Predicate<Lib3hServerProtocol>>);

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
}

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

const MAX_PROCESSING_LOOPS: u64 = 20;

#[allow(dead_code)]
/// Convenience function that asserts only one particular predicate
/// passes for a collection of engines. See assert_processed for
/// more information.
pub fn assert_one_processed(
    engines: &mut Vec<&mut Box<dyn NetworkEngine>>,
    processor: Box<dyn Processor>,
) -> Vec<ProcessorResult> {
    assert_processed(engines, &vec![processor])
}

// TODO Return back engines?
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
#[allow(dead_code)]
pub fn assert_processed(
    engines: &mut Vec<&mut Box<dyn NetworkEngine>>,
    processors: &Vec<Box<dyn Processor>>,
) -> Vec<ProcessorResult> {
    let mut previous = Vec::new();
    let mut errors = Vec::new();

    for p in processors {
        errors.push((p, None))
    }

    for epoch in 0..MAX_PROCESSING_LOOPS {
        println!("[{:?}] {:?}", epoch, previous);

        for engine in engines.iter_mut() {
            let (did_work, events) = engine
                .process()
                .map_err(|err| dbg!(err))
                .unwrap_or((false, vec![]));
            if events.is_empty() {
                continue;
            }

            let events = dbg!(events);
            let processor_result = ProcessorResult {
                did_work,
                events,
                engine_name: engine.name(),
                previous: previous.clone(),
            };
            let mut failed = Vec::new();

            for (processor, _orig_processor_result) in errors.drain(..) {
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
            errors.append(&mut failed);
            if !processor_result.events.is_empty() {
                previous.push(processor_result.clone());
            }

            if errors.is_empty() {
                break;
            }
        }
    }

    for (p, args) in errors {
        if let Some(args) = args {
            p.test(&args)
        } else {
            // Make degenerate result which should fail
            p.test(&ProcessorResult {
                engine_name: "none".into(),
                previous : vec![],
                events : vec![],
                did_work: false
            })
        }
    }
    previous
}

/// Creates a processor that verifies a connected data response is produced
/// by an engine
#[allow(dead_code)]
pub fn is_connected(request_id: &str, uri: url::Url) -> Lib3hServerProtocolEquals {
    Lib3hServerProtocolEquals(Lib3hServerProtocol::Connected(ConnectedData {
        request_id: request_id.into(),
        uri,
    }))
}

use predicates::prelude::*;

use lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_server::Lib3hServerProtocol,
};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ProcessorArgs {
    did_work: bool,
    engine_name: String,
    events: Vec<Lib3hServerProtocol>,
    previous: Vec<ProcessorArgs>,
}

pub trait Processor: Predicate<ProcessorArgs> {
    fn name(&self) -> String {
        "default_processor".into()
    }

    fn test(&self, args: &ProcessorArgs);
}

pub trait AssertEquals<T: PartialEq + std::fmt::Debug> {
    fn extracted(&self, args: &ProcessorArgs) -> Vec<T>;

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

impl<T> Predicate<ProcessorArgs> for dyn AssertEquals<T>
where
    T: PartialEq + std::fmt::Debug,
{
    fn eval(&self, args: &ProcessorArgs) -> bool {
        let extracted = self.extracted(args);
        extracted
            .iter()
            .find(|actual| **actual == self.expected())
            .is_some()
    }
}

pub trait Assert<T> {
    fn extracted(&self, args: &ProcessorArgs) -> Vec<T>;

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
    fn test(&self, args: &ProcessorArgs) {
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
    fn test(&self, args: &ProcessorArgs) {
        assert!(args.engine_name == self.0);
        assert!(args.did_work);
    }
}

impl Predicate<ProcessorArgs> for DidWorkAssert {
    fn eval(&self, args: &ProcessorArgs) -> bool {
        args.engine_name == self.0 && args.did_work
    }
}

impl Assert<Lib3hServerProtocol> for Lib3hServerProtocolAssert {
    fn extracted(&self, args: &ProcessorArgs) -> Vec<Lib3hServerProtocol> {
        args.events.iter().map(|x| x.clone()).collect()
    }

    fn assert_inner(&self, x: &Lib3hServerProtocol) -> bool {
        self.0.eval(&x)
    }
}

impl predicates::Predicate<ProcessorArgs> for Lib3hServerProtocolEquals {
    fn eval(&self, args: &ProcessorArgs) -> bool {
        self.extracted(args)
            .iter()
            .find(|actual| **actual == self.expected())
            .is_some()
    }
}

impl Predicate<ProcessorArgs> for Lib3hServerProtocolAssert {
    fn eval(&self, args: &ProcessorArgs) -> bool {
        let extracted = self.extracted(args);
        extracted
            .iter()
            .find(|actual| self.assert_inner(*actual))
            .is_some()
    }
}

impl Processor for Lib3hServerProtocolEquals {
    fn test(&self, args: &ProcessorArgs) {
        let extracted = self.extracted(args);
        let actual = extracted.iter().find(|actual| **actual == self.expected());
        assert_eq!(Some(&self.expected()), actual.or(extracted.first()));
    }
}

impl AssertEquals<Lib3hServerProtocol> for Lib3hServerProtocolEquals {
    fn extracted(&self, args: &ProcessorArgs) -> Vec<Lib3hServerProtocol> {
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
pub fn assert_one_processed(
    engines: &mut Vec<&mut Box<dyn NetworkEngine>>,
    processor: Box<dyn Processor>,
) {
    assert_processed(engines, &vec![processor])
}

// TODO Return back engines
// TODO return back processor events
#[allow(dead_code)]
pub fn assert_processed(
    engines: &mut Vec<&mut Box<dyn NetworkEngine>>,
    processors: &Vec<Box<dyn Processor>>,
) {
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
            let processor_args = ProcessorArgs {
                did_work,
                events,
                engine_name: engine.name(),
                previous: previous.clone(),
            };
            let mut failed = Vec::new();

            for (processor, _orig_processor_args) in errors.drain(..) {
                let result = processor.eval(&processor_args.clone());
                if result {
                    // Simulate the succesful assertion behavior
                    processor.test(&processor_args.clone());
                // processor passed!
                } else {
                    // Cache the assertion error and trigger it later if we never
                    // end up passing
                    failed.push((processor, Some(processor_args.clone())));
                }
            }
            errors.append(&mut failed);
            if !processor_args.events.is_empty() {
                previous.push(processor_args.clone());
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
            panic!(format!("Never tested processor: {}", p.name()))
        }
    }
}

#[allow(dead_code)]
pub fn is_connected(request_id: &str, uri: url::Url) -> Lib3hServerProtocolEquals {
    Lib3hServerProtocolEquals(Lib3hServerProtocol::Connected(ConnectedData {
        request_id: request_id.into(),
        uri,
    }))
}

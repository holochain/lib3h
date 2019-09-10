extern crate crossbeam_channel;
extern crate rustracing;
extern crate rustracing_jaeger;

pub type Span = rustracing_jaeger::Span;

/// Trait which enables a generic notion of tracing context, which is probably not necessary,
/// but was easy to do by hijacking the old TraceContext type parameter
pub trait CanTrace {
    fn get_span(&self) -> Span;
}

pub struct Lib3hTrace;
impl CanTrace for Lib3hTrace {
    fn get_span(&self) -> Span {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct TestTrace(pub String);

impl CanTrace for TestTrace {
    fn get_span(&self) -> Span {
        unimplemented!()
    }
}

impl TestTrace {
    pub fn new() -> Self {
        Self("Unnamed TestTrace".into())
    }
}

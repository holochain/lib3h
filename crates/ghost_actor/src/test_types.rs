use lib3h_tracing::{CanTrace, Span};

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

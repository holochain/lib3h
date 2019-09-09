use lib3h_tracing::{CanTrace, Span};

#[derive(Debug)]
pub struct TestContext(pub String);

impl CanTrace for TestContext {
    fn get_span(&self) -> Span {
        unimplemented!()
    }
}

impl TestContext {
    pub fn new() -> Self {
        Self("Unnamed TestContext".into())
    }
}

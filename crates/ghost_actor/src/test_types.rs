use crate::ghost_actor::CanTrace;
use lib3h_tracing::Span;

#[derive(Debug)]
pub struct TestContext(pub String);

impl CanTrace for TestContext {
    fn get_span(&self) -> Span {
        unimplemented!()
    }
}

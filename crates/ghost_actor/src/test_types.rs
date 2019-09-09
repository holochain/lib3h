use crate::ghost_actor::GhostContext;
use lib3h_tracing::Span;

#[derive(Debug)]
pub struct TestContext(pub String);

impl GhostContext for TestContext {
    fn get_span(&self) -> Span {
        unimplemented!()
    }
}

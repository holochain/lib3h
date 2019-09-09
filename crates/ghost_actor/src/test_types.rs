use crate::ghost_actor::GhostContext;

#[derive(Debug)]
pub struct TestContext(pub String);

impl GhostContext for TestContext {
    fn get_span(&self) {}
}

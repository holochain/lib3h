use crate::ghost_actor::GhostContext;

pub type TestError = String;

#[derive(Debug)]
pub struct TestCallbackData(pub String);

#[derive(Debug)]
pub struct TestContext(pub String);

impl GhostContext for TestContext {
    fn get_span(&self) {}
}

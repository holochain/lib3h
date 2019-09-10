extern crate crossbeam_channel;
extern crate rustracing;
extern crate rustracing_jaeger;
#[macro_use]
extern crate shrinkwraprs;

use rustracing::sampler::AllSampler;
use rustracing_jaeger::Tracer;
use std::{
    borrow::{Borrow, BorrowMut},
    ops::{Deref, DerefMut},
};

pub type Span = rustracing_jaeger::Span;

/// Trait which enables a generic notion of tracing context, which is probably not necessary,
/// but was easy to do by hijacking the old TraceContext type parameter
/// It requires all the trait implementations provided by a mutable shinkwrap
pub trait CanTrace:
    AsMut<Span>
    + BorrowMut<Span>
    + DerefMut<Target = Span>
    + AsRef<Span>
    + Borrow<Span>
    + Deref<Target = Span>
{
}

#[derive(Debug, Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct Lib3hTrace(pub Span);
impl CanTrace for Lib3hTrace {}

#[derive(Debug, Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct TestTrace {
    name: String,
    #[shrinkwrap(main_field)]
    pub span: Span,
}
impl CanTrace for TestTrace {}

impl TestTrace {
    pub fn new(name: &str) -> Self {
        let (tracer, _) = Tracer::new(AllSampler);
        let span = tracer.span("test").start();
        Self {
            name: name.into(),
            span: span,
        }
    }
}

impl Default for TestTrace {
    fn default() -> Self {
        Self::new("Unnamed TestTrace")
    }
}

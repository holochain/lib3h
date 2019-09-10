extern crate crossbeam_channel;
extern crate rustracing;
extern crate rustracing_jaeger;
#[macro_use]
extern crate shrinkwraprs;

use rustracing::sampler::AllSampler;
use rustracing_jaeger::Tracer;
use std::{
    borrow::{Borrow, BorrowMut, Cow},
    ops::{Deref, DerefMut},
};

pub type Span = rustracing_jaeger::Span;

/// Trait which enables a generic notion of tracing context, which is probably not necessary,
/// but was easy to do by hijacking the old TraceContext type parameter
/// It requires all the trait implementations provided by a mutable shinkwrap
pub trait CanTrace:
    From<Span>
    // The following are provided by Shrinkwrap (mutable)
    + AsMut<Span>
    + BorrowMut<Span>
    + DerefMut<Target = Span>
    + AsRef<Span>
    + Borrow<Span>
    + Deref<Target = Span>
{
    fn event<S: Into<Cow<'static, str>>>(&mut self, msg: S) {
        self.log(|l| {
            l.std().event(msg);
        })
    }

    fn error<S: Into<Cow<'static, str>>>(&mut self, kind: S, msg: S) {
        self.log(|l| {
            l.error().kind(kind).message(msg);
        })
    }

    fn child_span<S: Into<Cow<'static, str>>>(&self, operation_name: S) -> Self {
        self.child(operation_name, |o| o.start()).into()
    }
}

#[derive(Debug, Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct Lib3hTrace(pub Span);
impl CanTrace for Lib3hTrace {}
impl From<Span> for Lib3hTrace {
    fn from(span: Span) -> Lib3hTrace {
        Lib3hTrace(span)
    }
}

#[derive(Debug, Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct TestTrace {
    #[shrinkwrap(main_field)]
    pub span: Span,
    pub name: String,
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

impl From<Span> for TestTrace {
    fn from(span: Span) -> TestTrace {
        TestTrace {
            span,
            name: "Derived TestTrace".into(),
        }
    }
}

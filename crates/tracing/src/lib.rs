extern crate crossbeam_channel;
extern crate rustracing;
extern crate rustracing_jaeger;
#[macro_use]
extern crate shrinkwraprs;

use rustracing::sampler::AllSampler;
use std::borrow::Cow;

pub type Span = rustracing_jaeger::Span;
pub type Tracer = rustracing_jaeger::Tracer;

#[derive(Debug, Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct Lib3hSpan(pub Span);

impl From<Span> for Lib3hSpan {
    fn from(span: Span) -> Lib3hSpan {
        Lib3hSpan(span)
    }
}

impl Lib3hSpan {
    pub fn event<S: Into<Cow<'static, str>>>(&mut self, msg: S) {
        self.log(|l| {
            l.std().event(msg);
        })
    }

    pub fn error<S: Into<Cow<'static, str>>>(&mut self, kind: S, msg: S) {
        self.log(|l| {
            l.error().kind(kind).message(msg);
        })
    }

    pub fn child_span<S: Into<Cow<'static, str>>>(&self, operation_name: S) -> Self {
        self.child(operation_name, |o| o.start()).into()
    }

    pub fn todo() -> Self {
        test_span("TODO Span")
    }
}

pub fn test_span(name: &str) -> Lib3hSpan {
    Tracer::new(AllSampler)
        .0
        .span(name.to_owned())
        .start()
        .into()
}

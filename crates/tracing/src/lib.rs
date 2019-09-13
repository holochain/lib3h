#![feature(rustc_private)]

extern crate crossbeam_channel;
extern crate rustracing;
extern crate rustracing_jaeger;
#[macro_use]
extern crate shrinkwraprs;

use crate::rustracing::carrier::{ExtractFromBinary, InjectToBinary};
use std::{borrow::Cow, io::Cursor};

use rustracing_jaeger::span::SpanContextState;
pub use rustracing_jaeger::Result;

pub use rustracing::sampler::*;
pub use rustracing_jaeger::*;

pub type Span = rustracing_jaeger::Span;
pub type SpanContext = rustracing_jaeger::span::SpanContext;
pub type Tracer = rustracing_jaeger::Tracer;
pub type Reporter = rustracing_jaeger::reporter::JaegerCompactReporter;

#[derive(Debug, Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct Lib3hSpan(pub Span);

impl From<Span> for Lib3hSpan {
    fn from(span: Span) -> Lib3hSpan {
        Lib3hSpan(span)
    }
}

/// Binary representation is exactly 37 bytes, so ideally
/// we would use a [u8; 37], but this is easier...
pub type EncodedSpanContext = Vec<u8>;

/// An OpenTracing SpanContext is used to send span info across a process boundary
pub struct Lib3hSpanContext(pub SpanContext);

impl Lib3hSpanContext {
    /// Create a follower Span from this SpanContext
    /// NB: there is intentionally no method to create a child span from a context,
    /// since it's assumed that all inter-process points of a trace are async and
    /// the parent span will have ended before this one does
    pub fn follower<S: Into<Cow<'static, str>>>(
        &self,
        tracer: &Tracer,
        operation_name: S,
    ) -> Lib3hSpan {
        tracer
            .span(operation_name)
            .follows_from(&self.0)
            .start()
            .into()
    }

    /// Serialize to binary format for packing into a IPC message
    pub fn encode(&self) -> Result<EncodedSpanContext> {
        let mut enc: Vec<u8> = [0; 37].to_vec(); // OpenTracing binary format is 37 bytes
        let mut slice = &mut enc[..];
        SpanContextState::inject_to_binary(&self.0, &mut slice)?;
        Ok(enc)
    }

    /// Deserialize from binary format
    pub fn decode(enc: &EncodedSpanContext) -> Result<Self> {
        let mut cursor = Cursor::new(enc);
        SpanContextState::extract_from_binary(&mut cursor).map(|x| Lib3hSpanContext(x.unwrap()))
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

    pub fn context(&self) -> Option<Lib3hSpanContext> {
        self.0.context().map(|ctx| Lib3hSpanContext(ctx.to_owned()))
    }
    // // FnOnce<(rustracing::span::StartSpanOptions<'_, rustracing::sampler::AllSampler, rustracing_jaeger::span::SpanContextState>,)
    // pub fn child_<S: Into<Cow<'static, str>>, F>(&self, operation_name: S, f: F) -> StartSpanOptions
    // where
    //     F: FnOnce(StartSpanOptions<'_, _, _> -> StartSpanOptions<'_, _, _>),
    // {
    //     self.0.child(operation_name, f)
    // }

    pub fn child<S: Into<Cow<'static, str>>>(&self, operation_name: S) -> Self {
        self.0.child(operation_name, |o| o.start()).into()
    }

    pub fn follower<S: Into<Cow<'static, str>>>(&self, operation_name: S) -> Self {
        self.0.follower(operation_name, |o| o.start()).into()
    }

    pub fn todo() -> Self {
        test_span("TODO: no-op, disconnected Span")
    }
}

pub fn test_span(name: &str) -> Lib3hSpan {
    Tracer::new(AllSampler)
        .0
        .span(name.to_owned())
        .start()
        .into()
}

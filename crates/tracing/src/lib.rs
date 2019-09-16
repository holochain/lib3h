#![feature(rustc_private)]

extern crate crossbeam_channel;
extern crate rustracing;
extern crate rustracing_jaeger;
#[macro_use]
extern crate shrinkwraprs;

use crate::rustracing::carrier::{ExtractFromBinary, InjectToBinary};
use rustracing::span::StartSpanOptions;
use rustracing_jaeger::span::SpanContextState;
use std::{borrow::Cow, io::Cursor};

pub use rustracing::sampler::*;
pub use rustracing_jaeger::{Result, *};

pub type Span = rustracing_jaeger::Span;
pub type SpanContext = rustracing_jaeger::span::SpanContext;
pub type Tracer = rustracing_jaeger::Tracer;
pub type Reporter = rustracing_jaeger::reporter::JaegerCompactReporter;

/// A wrapper around a simple rustracing_jaeger::Span, providing some
/// convenience functions.
/// It overshadows the lower-level `child` and `follower` methods
/// with simpler versions. To access the lower-level methods, use `.0`.
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

    pub fn context(&self) -> Option<Lib3hSpanContext> {
        self.0.context().map(|ctx| Lib3hSpanContext(ctx.to_owned()))
    }

    /// Renaming of underlying `child` method
    pub fn child_<'a, N: Into<Cow<'static, str>>, F, C, T>(
        &'a self,
        operation_name: N,
        f: F,
    ) -> Span
    where
        F: FnOnce(StartSpanOptions<'_, AllSampler, SpanContextState>) -> Span,
    {
        self.0.child(operation_name, f)
    }

    /// Renaming of underlying `follow` method
    pub fn follower_<'a, N: Into<Cow<'static, str>>, F, C, T>(
        &'a self,
        operation_name: N,
        f: F,
    ) -> Span
    where
        F: FnOnce(StartSpanOptions<'_, AllSampler, SpanContextState>) -> Span,
    {
        self.0.follower(operation_name, f)
    }

    /// Call underlying `child` method with only a simple operation name
    pub fn child<S: Into<Cow<'static, str>>>(&self, operation_name: S) -> Self {
        self.0.child(operation_name, |o| o.start()).into()
    }

    /// Call underlying `follower` method with only a simple operation name
    pub fn follower<S: Into<Cow<'static, str>>>(&self, operation_name: S) -> Self {
        self.0.follower(operation_name, |o| o.start()).into()
    }

    /// Wrap this span in a SpanWrap along with some user data
    pub fn wrap<T>(self, data: T) -> SpanWrap<T> {
        SpanWrap::new(data, self)
    }

    /// e.g. for times when a function requires a Span but we don't desire to actually
    /// instrument that function call.
    pub fn noop() -> Self {
        noop("no-op, intentionally disconnected Span".into())
    }

    /// Useful for retrofitting existing codebases with traces. Use this to
    /// create a valid span that is not hooked up to any actual tracer, but which
    /// at least provides some visibility into something that needs fixing
    pub fn todo(reason: &'static str) -> Self {
        noop(format!("TODO: {}", reason))
    }

    /// TODO: remove all of these. This is a more extreme version of todo()
    pub fn fixme() -> Self {
        noop("not yet hooked up".into())
    }
}

/// SpanWrap is a simple way to couple some data along with a struct It is
/// common to send some data on a channel which will be used as arguments
/// to a function on the receiving side, where we also want to continue the
/// trace on the receiving side. This struct helps keep that data together
/// with minimal boilerplate.
///
/// The use of shrinkwrap allows the entire struct to be used as if it were
/// a bare T (in most situations), but the Span can also be extracted.
#[derive(Shrinkwrap)]
#[shrinkwrap(mutable)]
pub struct SpanWrap<T> {
    #[shrinkwrap(main_field)]
    pub data: T,
    pub span: Lib3hSpan,
}

impl<T> SpanWrap<T> {
    pub fn new(data: T, span: Lib3hSpan) -> Self {
        Self { data, span }
    }
}

/// Binary representation is exactly 37 bytes, so ideally
/// we would use a [u8; 37], but this is easier...
pub type EncodedSpanContext = Vec<u8>;

/// An OpenTracing SpanContext is used to send span info across a process
/// boundary. This is a simple wrapper around that, again with some helper
/// functions.
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

/// TODO: use lazy_static / thread_local singleton Tracer
fn noop(name: String) -> Lib3hSpan {
    Tracer::new(NullSampler).0.span(name).start().into()
}

/// Dummy span, useful for tests that don't test tracing
pub fn test_span(name: &str) -> Lib3hSpan {
    noop(name.into())
}

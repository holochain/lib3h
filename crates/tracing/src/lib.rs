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

// 296 bytes total == 8 * 37, so ideally we would use a [u8; 37]
// but this is easier...
pub type EncodedSpanContext = Vec<u8>;

pub struct IpcSpanContext(pub SpanContext);

impl IpcSpanContext {
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

    pub fn encode(&self) -> Result<EncodedSpanContext> {
        let mut enc: Vec<u8> = [0; 37].to_vec(); // OpenTracing binary format is 37 bytes
        let mut slice = &mut enc[..];
        SpanContextState::inject_to_binary(&self.0, &mut slice)?;
        Ok(enc)
    }

    pub fn decode(enc: &EncodedSpanContext) -> Result<Self> {
        let mut cursor = Cursor::new(enc);
        SpanContextState::extract_from_binary(&mut cursor).map(|x| IpcSpanContext(x.unwrap()))
    }
}

// impl Serialize for IpcSpanContext {
//     fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let bytes: Vec<u8> = self.encode().map_err(|e| SerError::custom(e.to_string()))?;
//         serializer.serialize_bytes(&bytes)
//     }
// }

// impl<'de> Deserialize<'de> for IpcSpanContext {
//     fn deserialize<D>(deserializer: D) -> Result<IpcSpanContext, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         deserializer.deserialize_bytes()
//     }
// }

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

    pub fn context(&self) -> Option<IpcSpanContext> {
        self.0.context().map(|ctx| IpcSpanContext(ctx.to_owned()))
    }

    pub fn child<S: Into<Cow<'static, str>>>(&self, operation_name: S) -> Self {
        self.0.child(operation_name, |o| o.start()).into()
    }

    pub fn follower<S: Into<Cow<'static, str>>>(&self, operation_name: S) -> Self {
        self.0.follower(operation_name, |o| o.start()).into()
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

//! File-based streams for the Effect ecosystem.
//!
//! This package provides file-based streams that implement the Effect ecosystem's
//! stream traits. It includes both producers (for reading files) and consumers
//! (for writing to files).

mod consumer;
mod error;
mod producer;

pub use consumer::FileConsumer;
pub use error::FileStreamError;
pub use producer::FileProducer;

/// Re-exports from effect-stream for convenience
pub use effect_stream::{EffectResult, EffectStream, EffectStreamSink, EffectStreamSource};

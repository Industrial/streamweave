//! Command execution streams for the Effect ecosystem.
//!
//! This package provides streams that interact with external commands. It includes
//! both producers (for reading command output) and consumers (for writing to
//! command input).

mod consumer;
mod error;
mod producer;

pub use consumer::CommandConsumer;
pub use error::CommandStreamError;
pub use producer::CommandProducer;

/// Re-exports from effect-stream for convenience
pub use effect_stream::{EffectResult, EffectStream, EffectStreamSink, EffectStreamSource};

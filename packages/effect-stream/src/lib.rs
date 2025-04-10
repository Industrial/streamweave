//! Streaming capabilities for the Effect type system.
//!
//! This module provides the `EffectStream` type, which represents a stream of values
//! that can be transformed and combined using the Effect type system's functor,
//! applicative, and monad operations.

pub mod stream;

pub use stream::EffectStream;

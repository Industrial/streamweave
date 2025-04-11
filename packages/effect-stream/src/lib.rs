#![recursion_limit = "256"]

//! A stream implementation for handling effects and errors.
//!
//! This crate provides an implementation of streams that can handle effects and errors:
//! - `EffectStream`: A stream that can produce values and handle errors
//! - Stream-specific error handling
//! - Stream operations and utilities

pub mod error;
pub mod stream;

pub use error::{EffectError, EffectResult};
pub use stream::EffectStream;

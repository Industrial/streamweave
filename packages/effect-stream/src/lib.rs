#![recursion_limit = "256"]

//! A stream implementation for handling effects and errors.
//!
//! This crate provides an implementation of streams that can handle effects and errors:
//! - `EffectStream`: A stream that can produce values and handle errors
//! - Stream-specific error handling
//! - Stream operations and utilities

pub mod error;
// pub mod operator;
// pub mod sink;
pub mod source;
pub mod stream;

pub use error::*;
// pub use operator::*;
// pub use sink::*;
pub use source::*;
pub use stream::*;

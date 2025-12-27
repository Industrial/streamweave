//! Core traits and types for StreamWeave
//!
//! This package provides the foundational traits and types that all other
//! StreamWeave packages depend on.

pub mod consumer;
pub mod input;
pub mod output;
pub mod port;
pub mod producer;
pub mod transformer;

pub use consumer::*;
pub use input::*;
pub use output::*;
pub use port::*;
pub use producer::*;
pub use transformer::*;

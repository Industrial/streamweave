//! Machine learning inference support for StreamWeave
//!
//! This module provides traits and implementations for ML inference backends,
//! enabling integration with various ML frameworks.

pub mod hotswap;
pub mod inference_backend;
pub mod onnx;

pub use hotswap::*;
pub use inference_backend::*;
pub use onnx::*;

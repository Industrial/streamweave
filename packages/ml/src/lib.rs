//! ML transformers for StreamWeave
//!
//! This package provides machine learning inference transformers for StreamWeave pipelines.
//! It includes support for ONNX Runtime and provides a trait-based abstraction for
//! adding support for other ML frameworks.

pub mod batched_inference_transformer;
pub mod hotswap;
pub mod inference_backend;
pub mod inference_transformer;
#[cfg(feature = "onnx")]
pub mod onnx;

pub use batched_inference_transformer::*;
pub use hotswap::*;
pub use inference_backend::*;
pub use inference_transformer::*;
#[cfg(feature = "onnx")]
pub use onnx::*;

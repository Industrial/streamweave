//! # Machine Learning Module
//!
//! Module that provides machine learning inference support for StreamWeave, enabling
//! integration with various ML frameworks for real-time inference in streaming pipelines.
//!
//! This module provides traits and implementations for ML inference backends,
//! enabling integration with various ML frameworks. It supports hot-swappable
//! model loading, ONNX runtime integration, and batched inference for efficient
//! processing.
//!
//! # Overview
//!
//! The ML module enables real-time machine learning inference in StreamWeave pipelines
//! and graphs. It provides a flexible backend system that supports multiple ML frameworks,
//! hot-swappable models, and efficient batched inference. Perfect for integrating ML
//! models into streaming data processing workflows.
//!
//! # Key Concepts
//!
//! - **Inference Backend**: Trait-based system for ML framework integration
//! - **Hot-Swappable Models**: Support for updating models without stopping processing
//! - **ONNX Runtime**: Integration with ONNX Runtime for model inference
//! - **Batched Inference**: Efficient processing of multiple inputs in batches
//! - **Framework Agnostic**: Trait-based design supporting multiple ML frameworks
//!
//! # Core Types
//!
//! - **[`InferenceBackend`]**: Trait for ML inference backends
//! - **[`crate::ml::onnx::OnnxBackend`]**: ONNX Runtime inference backend
//! - **[`hotswap`]**: Hot-swappable backend implementation
//!
//! # Quick Start
//!
//! ## Basic Usage with ONNX Runtime
//!
//! ```rust,no_run
//! use streamweave::ml::OnnxRuntime;
//! use streamweave::ml::InferenceBackend;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Load an ONNX model
//! let backend = OnnxRuntime::new("model.onnx").await?;
//!
//! // Run inference
//! let input = vec![1.0, 2.0, 3.0];
//! let output = backend.infer(&input).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Hot-Swappable Backend
//!
//! ```rust,no_run
//! use streamweave::ml::{HotSwapBackend, OnnxRuntime};
//! use streamweave::ml::InferenceBackend;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a hot-swappable backend
//! let mut backend = HotSwapBackend::new(
//!     OnnxRuntime::new("model.onnx").await?
//! );
//!
//! // Run inference
//! let input = vec![1.0, 2.0, 3.0];
//! let output = backend.infer(&input).await?;
//!
//! // Hot-swap to a new model without stopping processing
//! backend.swap(OnnxRuntime::new("new_model.onnx").await?)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Batched Inference
//!
//! ```rust,no_run
//! use streamweave::ml::OnnxRuntime;
//! use streamweave::ml::InferenceBackend;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let backend = OnnxRuntime::new("model.onnx").await?;
//!
//! // Run batched inference for efficiency
//! let inputs = vec![
//!     vec![1.0, 2.0, 3.0],
//!     vec![4.0, 5.0, 6.0],
//!     vec![7.0, 8.0, 9.0],
//! ];
//! let outputs = backend.infer_batch(&inputs).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Design Decisions
//!
//! - **Trait-Based Design**: Uses traits for framework-agnostic inference
//! - **Hot-Swappable Models**: Supports model updates without stopping processing
//! - **ONNX Support**: Primary support for ONNX Runtime for cross-framework compatibility
//! - **Batched Processing**: Efficient batch processing for high-throughput scenarios
//! - **Async Inference**: All inference operations are async for non-blocking execution
//!
//! # Integration with StreamWeave
//!
//! The ML module integrates with StreamWeave through transformers and graph nodes.
//! ML inference can be performed inline in pipelines and graphs, enabling real-time
//! ML processing in streaming workflows. See the `ml_inference_transformer` module
//! and [`crate::transformers::BatchedInferenceTransformer`] for usage examples.
//!
//! # Submodules
//!
//! - **[`inference_backend`]**: Trait definitions for ML inference backends
//! - **[`onnx`]**: ONNX Runtime integration
//! - **[`hotswap`]**: Hot-swappable backend implementation

pub mod hotswap;
pub mod inference_backend;
pub mod onnx;

pub use crate::error::{ErrorAction, ErrorContext, ErrorStrategy, StreamError};
pub use hotswap::*;
pub use inference_backend::*;
pub use onnx::*;

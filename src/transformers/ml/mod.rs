//! # Machine Learning Integration
//!
//! This module provides machine learning integration for StreamWeave pipelines,
//! enabling real-time model inference on streaming data.
//!
//! ## Core Concepts
//!
//! - **Inference Backend**: A trait for ML model backends (ONNX, TensorFlow, etc.)
//! - **Inference Transformer**: A transformer that runs ML model inference on stream items
//! - **Batching**: Grouping multiple items for efficient batch inference
//! - **Hot-Swapping**: Replacing models at runtime without restarting the pipeline
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::prelude::*;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pipeline = Pipeline::new()
//!     .with_producer(ArrayProducer::new(vec![/* feature vectors */]))
//!     .with_transformer(OnnxInferenceTransformer::from_path("model.onnx")?)
//!     .with_consumer(VecConsumer::new());
//!
//! let (_, consumer) = pipeline.run().await?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "ml")]
pub mod batched_inference_transformer;
#[cfg(feature = "ml")]
pub mod hotswap;
#[cfg(feature = "ml")]
pub mod inference_backend;
#[cfg(feature = "ml")]
pub mod inference_transformer;
#[cfg(feature = "ml")]
pub mod onnx;

#[cfg(feature = "ml")]
pub use inference_backend::InferenceBackend;

#[cfg(feature = "ml")]
pub use onnx::{OnnxBackend, OnnxError};

#[cfg(feature = "ml")]
pub use batched_inference_transformer::{BatchedInferenceConfig, BatchedInferenceTransformer};

#[cfg(feature = "ml")]
pub use inference_transformer::InferenceTransformer;

#[cfg(feature = "ml")]
pub use hotswap::{SwapError, swap_model};

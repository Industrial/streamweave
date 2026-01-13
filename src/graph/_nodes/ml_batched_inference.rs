//! ML batched inference node for performing batched ML model inference.
//!
//! This module provides [`MlBatchedInference`], a graph node that performs batched
//! ML model inference operations. It batches inference requests for efficient
//! processing, especially important for GPU-accelerated inference. It wraps
//! [`BatchedInferenceTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`MlBatchedInference`] is useful for performing batched machine learning model
//! inference in graph-based pipelines. It batches multiple items together for
//! efficient processing, making it ideal for high-throughput inference scenarios
//! where GPU acceleration is available.
//!
//! # Key Concepts
//!
//! - **Batched Inference**: Batches multiple items together for efficient processing
//! - **GPU Optimization**: Especially important for GPU-accelerated inference
//! - **Configurable Batching**: Supports configurable batch size and timeout
//! - **Transformer Wrapper**: Wraps `BatchedInferenceTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`MlBatchedInference<B>`]**: Node that performs batched ML inference
//! - **[`InferenceBackend`]**: Trait for ML inference backends
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use streamweave::graph::nodes::MlBatchedInference;
//! use streamweave_ml_transformers::OnnxBackend;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create and load a model backend
//! let mut backend = OnnxBackend::new()?;
//! backend.load_from_path("model.onnx").await?;
//!
//! // Create a batched inference node
//! let inference = MlBatchedInference::new(backend)
//!     .with_batch_size(32)
//!     .with_timeout(Duration::from_millis(100));
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust,no_run
//! use streamweave::graph::nodes::MlBatchedInference;
//! use streamweave::ErrorStrategy;
//! use streamweave_ml_transformers::OnnxBackend;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let mut backend = OnnxBackend::new()?;
//! // Create a batched inference node with error handling
//! let inference = MlBatchedInference::new(backend)
//!     .with_batch_size(64)
//!     .with_timeout(Duration::from_millis(50))
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("batched-inference".to_string());
//! # Ok(())
//! # }
//! ```
//!
//! # Design Decisions
//!
//! - **Backend Abstraction**: Uses `InferenceBackend` trait for flexible backend support
//! - **Batched Processing**: Processes items in batches for efficient GPU utilization
//! - **Configurable Batching**: Supports batch size and timeout for flexible batching
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`MlBatchedInference`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::ml::InferenceBackend;
use crate::transformers::BatchedInferenceTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use std::time::Duration;

/// Node for batched ML model inference operations.
///
/// This node wraps `BatchedInferenceTransformer` for use in graphs. It batches
/// inference requests for efficient processing, especially important for
/// GPU-accelerated inference.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{MlBatchedInference, TransformerNode};
/// use streamweave_ml_transformers::OnnxBackend;
/// use std::time::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut backend = OnnxBackend::new()?;
/// backend.load_from_path("model.onnx").await?;
/// let inference = MlBatchedInference::new(backend)
///     .with_batch_size(32)
///     .with_timeout(Duration::from_millis(100));
/// let node = TransformerNode::from_transformer(
///     "batched_inference".to_string(),
///     inference,
/// );
/// # Ok(())
/// # }
/// ```
pub struct MlBatchedInference<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying batched inference transformer
  transformer: BatchedInferenceTransformer<B>,
}

impl<B> MlBatchedInference<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `MlBatchedInference` node with the specified backend.
  ///
  /// # Arguments
  ///
  /// * `backend` - The inference backend to use for batch inference.
  pub fn new(backend: B) -> Self {
    Self {
      transformer: BatchedInferenceTransformer::new(backend),
    }
  }

  /// Sets the batch size.
  ///
  /// # Arguments
  ///
  /// * `batch_size` - Maximum number of items per batch.
  pub fn with_batch_size(mut self, batch_size: usize) -> Self {
    self.transformer = self.transformer.with_batch_size(batch_size);
    self
  }

  /// Sets the timeout for partial batches.
  ///
  /// # Arguments
  ///
  /// * `timeout` - Maximum time to wait for a full batch before processing partial batch.
  pub fn with_timeout(mut self, timeout: Duration) -> Self {
    self.transformer = self.transformer.with_timeout(timeout);
    self
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<B::Input>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl<B> Input for MlBatchedInference<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = B::Input;
  type InputStream = Pin<Box<dyn Stream<Item = B::Input> + Send>>;
}

impl<B> Output for MlBatchedInference<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = B::Output;
  type OutputStream = Pin<Box<dyn Stream<Item = B::Output> + Send>>;
}

#[async_trait]
impl<B> Transformer for MlBatchedInference<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (B::Input,);
  type OutputPorts = (B::Output,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<B::Input>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<B::Input> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<B::Input> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<B::Input>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<B::Input>) -> ErrorContext<B::Input> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}

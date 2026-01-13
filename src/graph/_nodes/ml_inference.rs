//! ML inference node for performing single-item ML model inference.
//!
//! This module provides [`MlInference`], a graph node that performs single-item
//! ML model inference operations. It wraps an [`InferenceBackend`] and applies
//! inference to each item in the stream. It wraps [`InferenceTransformer`] for
//! use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`MlInference`] is useful for performing machine learning model inference in
//! graph-based pipelines. It applies inference to each item individually, making
//! it ideal for real-time inference scenarios where low latency is important.
//!
//! # Key Concepts
//!
//! - **Single-Item Inference**: Applies inference to each item individually
//! - **Inference Backend**: Uses an `InferenceBackend` trait for flexible backend support
//! - **Real-Time Processing**: Suitable for low-latency inference scenarios
//! - **Transformer Wrapper**: Wraps `InferenceTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`MlInference<B>`]**: Node that performs single-item ML inference
//! - **[`InferenceBackend`]**: Trait for ML inference backends
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use streamweave::graph::nodes::MlInference;
//! use streamweave_ml_transformers::OnnxBackend;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create and load a model backend
//! let mut backend = OnnxBackend::new()?;
//! backend.load_from_path("model.onnx").await?;
//!
//! // Create an ML inference node
//! let inference = MlInference::new(backend);
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust,no_run
//! use streamweave::graph::nodes::MlInference;
//! use streamweave::ErrorStrategy;
//! use streamweave_ml_transformers::OnnxBackend;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let mut backend = OnnxBackend::new()?;
//! // Create an ML inference node with error handling
//! let inference = MlInference::new(backend)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("ml-inference".to_string());
//! # Ok(())
//! # }
//! ```
//!
//! # Design Decisions
//!
//! - **Backend Abstraction**: Uses `InferenceBackend` trait for flexible backend support
//! - **Single-Item Processing**: Processes items individually for low-latency inference
//! - **Generic Backend**: Supports various ML backends through the trait system
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`MlInference`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::ml::InferenceBackend;
use crate::transformers::InferenceTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node for single-item ML model inference operations.
///
/// This node wraps `InferenceTransformer` for use in graphs. It applies
/// inference to each item in the stream using the provided inference backend.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{MlInference, TransformerNode};
/// use streamweave_ml_transformers::OnnxBackend;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut backend = OnnxBackend::new()?;
/// backend.load_from_path("model.onnx").await?;
/// let inference = MlInference::new(backend);
/// let node = TransformerNode::from_transformer(
///     "inference".to_string(),
///     inference,
/// );
/// # Ok(())
/// # }
/// ```
pub struct MlInference<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying inference transformer
  transformer: InferenceTransformer<B>,
}

impl<B> MlInference<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `MlInference` node with the specified backend.
  ///
  /// # Arguments
  ///
  /// * `backend` - The inference backend to use for model inference.
  pub fn new(backend: B) -> Self {
    Self {
      transformer: InferenceTransformer::new(backend),
    }
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

impl<B> Input for MlInference<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = B::Input;
  type InputStream = Pin<Box<dyn Stream<Item = B::Input> + Send>>;
}

impl<B> Output for MlInference<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = B::Output;
  type OutputStream = Pin<Box<dyn Stream<Item = B::Output> + Send>>;
}

#[async_trait]
impl<B> Transformer for MlInference<B>
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

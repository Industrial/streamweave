//! # Inference Transformer
//!
//! This module provides transformers that wrap inference backends for use in StreamWeave pipelines.
//!
//! ## Example
//!
//! ```rust,no_run
//! use crate::prelude::*;
//! use streamweave_ml_transformers::{OnnxBackend, InferenceTransformer};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut backend = OnnxBackend::new()?;
//! backend.load_from_path("model.onnx").await?;
//!
//! let pipeline = Pipeline::new()
//!     .with_producer(ArrayProducer::new(vec![vec![1.0, 2.0], vec![3.0, 4.0]]))
//!     .with_transformer(InferenceTransformer::new(backend))
//!     .with_consumer(VecConsumer::new());
//!
//! let (_, consumer) = pipeline.run().await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::ml::InferenceBackend;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_stream::stream;
use async_trait::async_trait;
use chrono;
use futures::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Transformer for single-item ML model inference operations.
///
/// This transformer wraps an `InferenceBackend` and applies inference to each
/// item in the stream individually. It's suitable for low-latency inference
/// where items arrive one at a time.
///
/// # Type Parameters
///
/// * `B` - The inference backend type that implements `InferenceBackend`
///
/// # Example
///
/// ```rust,no_run
/// use streamweave_ml_transformers::{InferenceTransformer, OnnxBackend};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut backend = OnnxBackend::new()?;
/// backend.load_from_path("model.onnx").await?;
///
/// let transformer = InferenceTransformer::new(backend);
/// # Ok(())
/// # }
/// ```
pub struct InferenceTransformer<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The inference backend
  backend: Arc<RwLock<B>>,
  /// Transformer configuration
  transformer_config: TransformerConfig<B::Input>,
}

impl<B> InferenceTransformer<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new inference transformer with the specified backend.
  ///
  /// # Arguments
  ///
  /// * `backend` - The inference backend to use for model inference
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave_ml_transformers::{InferenceTransformer, OnnxBackend};
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut backend = OnnxBackend::new()?;
  /// backend.load_from_path("model.onnx").await?;
  /// let transformer = InferenceTransformer::new(backend);
  /// # Ok(())
  /// # }
  /// ```
  pub fn new(backend: B) -> Self {
    Self {
      backend: Arc::new(RwLock::new(backend)),
      transformer_config: TransformerConfig::default(),
    }
  }

  /// Sets the error strategy for the transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<B::Input>) -> Self {
    self.transformer_config.error_strategy = strategy;
    self
  }

  /// Sets the name for the transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name for this transformer instance
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer_config.name = Some(name);
    self
  }

  /// Gets a reference to the backend for hot-swapping.
  ///
  /// # Returns
  ///
  /// An `Arc<RwLock<B>>` that can be used to replace the backend at runtime.
  pub fn backend(&self) -> Arc<RwLock<B>> {
    Arc::clone(&self.backend)
  }
}

impl<B> Input for InferenceTransformer<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = B::Input;
  type InputStream = Pin<Box<dyn futures::Stream<Item = B::Input> + Send>>;
}

impl<B> Output for InferenceTransformer<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = B::Output;
  type OutputStream = Pin<Box<dyn futures::Stream<Item = B::Output> + Send>>;
}

#[async_trait]
impl<B> Transformer for InferenceTransformer<B>
where
  B: InferenceBackend + 'static,
  B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (B::Input,);
  type OutputPorts = (B::Output,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let backend = Arc::clone(&self.backend);
    let error_strategy = self.transformer_config.error_strategy.clone();
    let component_name = self
      .transformer_config
      .name
      .clone()
      .unwrap_or_else(|| "InferenceTransformer".to_string());

    Box::pin(stream! {
      let mut input = input;
      while let Some(item) = input.next().await {
        let backend_guard = backend.read().await;
        let item_clone = item.clone();
        match backend_guard.infer(item).await {
          Ok(output) => yield output,
          Err(e) => {
            drop(backend_guard);
            let stream_error = StreamError::new(
              Box::new(e) as Box<dyn std::error::Error + Send + Sync>,
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(item_clone),
                component_name: component_name.clone(),
                component_type: std::any::type_name::<InferenceTransformer<B>>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<InferenceTransformer<B>>().to_string(),
              },
            );
            match handle_error(&error_strategy, &stream_error) {
              ErrorAction::Stop => break,
              ErrorAction::Skip => continue,
              ErrorAction::Retry => {
                // For retry, we could implement exponential backoff, but for now just skip
                continue;
              }
            }
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<B::Input>) {
    self.transformer_config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<B::Input> {
    &self.transformer_config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<B::Input> {
    &mut self.transformer_config
  }

  fn handle_error(&self, error: &StreamError<B::Input>) -> ErrorAction {
    handle_error(&self.transformer_config.error_strategy, error)
  }

  fn create_error_context(&self, item: Option<B::Input>) -> ErrorContext<B::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .transformer_config
        .name
        .clone()
        .unwrap_or_else(|| "InferenceTransformer".to_string()),
      component_type: std::any::type_name::<InferenceTransformer<B>>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo::new(
      self
        .transformer_config
        .name
        .clone()
        .unwrap_or_else(|| "InferenceTransformer".to_string()),
      "transformer".to_string(),
    )
  }
}

fn handle_error<M: std::fmt::Debug + Clone + Send + Sync>(
  strategy: &ErrorStrategy<M>,
  error: &StreamError<M>,
) -> ErrorAction {
  match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    ErrorStrategy::Skip => ErrorAction::Skip,
    ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}

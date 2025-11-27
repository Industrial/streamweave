//! # Inference Transformer
//!
//! This module provides transformers that wrap inference backends for use in StreamWeave pipelines.
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::prelude::*;
//! use streamweave::transformers::ml::{OnnxBackend, InferenceTransformer};
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

use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use crate::transformers::ml::InferenceBackend;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Transformer that runs ML inference on stream items.
///
/// This transformer wraps an `InferenceBackend` and applies inference to each
/// item in the stream. It supports both single-item and batch inference modes.
///
/// # Type Parameters
///
/// * `B` - The inference backend type
/// * `I` - The input type (typically the same as `B::Input`)
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::ml::{InferenceTransformer, OnnxBackend};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut backend = OnnxBackend::new()?;
/// backend.load_from_path("model.onnx").await?;
///
/// let transformer = InferenceTransformer::new(backend);
/// // Use in pipeline...
/// # Ok(())
/// # }
/// ```
pub struct InferenceTransformer<B>
where
  B: InferenceBackend + 'static,
{
  /// The inference backend
  backend: Arc<RwLock<B>>,
  /// Transformer configuration
  config: TransformerConfig<B::Input>,
  /// Phantom data marker
  _phantom: PhantomData<B::Input>,
}

impl<B> InferenceTransformer<B>
where
  B: InferenceBackend + 'static,
{
  /// Creates a new inference transformer with the given backend.
  ///
  /// # Arguments
  ///
  /// * `backend` - The inference backend to use for model inference
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave::transformers::ml::{InferenceTransformer, OnnxBackend};
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
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }

  /// Sets the error strategy for the transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave::transformers::ml::{InferenceTransformer, OnnxBackend};
  /// # use streamweave::error::ErrorStrategy;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut backend = OnnxBackend::new()?;
  /// backend.load_from_path("model.onnx").await?;
  /// let transformer = InferenceTransformer::new(backend)
  ///     .with_error_strategy(ErrorStrategy::Skip);
  /// # Ok(())
  /// # }
  /// ```
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<B::Input>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name for this transformer instance
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Gets a reference to the backend for hot-swapping.
  ///
  /// # Returns
  ///
  /// An `Arc<RwLock<B>>` that can be used to replace the backend at runtime.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave::transformers::ml::{InferenceTransformer, OnnxBackend, swap_model};
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut backend = OnnxBackend::new()?;
  /// backend.load_from_path("model.onnx").await?;
  ///
  /// let transformer = InferenceTransformer::new(backend);
  /// let backend_ref = transformer.backend();
  ///
  /// // Later, swap to a new model
  /// let mut new_backend = OnnxBackend::new()?;
  /// new_backend.load_from_path("new_model.onnx").await?;
  /// swap_model(&backend_ref, new_backend).await?;
  /// # Ok(())
  /// # }
  /// ```
  pub fn backend(&self) -> Arc<RwLock<B>> {
    Arc::clone(&self.backend)
  }

  /// Hot-swaps the model by replacing the backend with a new one.
  ///
  /// This is a convenience method that wraps the `swap_model` function.
  /// The new backend must have its model loaded before calling this method.
  ///
  /// # Arguments
  ///
  /// * `new_backend` - The new backend instance with the updated model
  ///
  /// # Returns
  ///
  /// Returns `Ok(())` if the swap was successful.
  ///
  /// # Errors
  ///
  /// Returns an error if the new backend is not loaded.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave::transformers::ml::{InferenceTransformer, OnnxBackend};
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut backend = OnnxBackend::new()?;
  /// backend.load_from_path("model.onnx").await?;
  ///
  /// let mut transformer = InferenceTransformer::new(backend);
  ///
  /// // Later, swap to a new model
  /// let mut new_backend = OnnxBackend::new()?;
  /// new_backend.load_from_path("new_model.onnx").await?;
  /// transformer.swap_backend(new_backend).await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn swap_backend(
    &mut self,
    new_backend: B,
  ) -> Result<(), crate::transformers::ml::SwapError> {
    crate::transformers::ml::swap_model(&self.backend, new_backend).await
  }
}

// Transformer trait implementation
#[cfg(feature = "ml")]
mod transformer_impl {
  use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
  use crate::input::Input;
  use crate::output::Output;
  use crate::transformer::{Transformer, TransformerConfig};
  use crate::transformers::ml::{InferenceBackend, InferenceTransformer};
  use async_trait::async_trait;
  use futures::StreamExt;
  use std::pin::Pin;
  use std::sync::Arc;

  // Input trait implementation
  impl<B> Input for InferenceTransformer<B>
  where
    B: InferenceBackend + 'static,
    B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  {
    type Input = B::Input;
    type InputStream = Pin<Box<dyn futures::Stream<Item = B::Input> + Send>>;
  }

  // Output trait implementation
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
    fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
      let backend = Arc::clone(&self.backend);
      Box::pin(
        input
          .then(move |item| {
            let backend = Arc::clone(&backend);
            async move {
              let backend_guard = backend.read().await;
              match backend_guard.infer(item).await {
                Ok(output) => Some(output),
                Err(e) => {
                  eprintln!("Inference error: {}", e);
                  None
                }
              }
            }
          })
          .filter_map(futures::future::ready),
      )
    }

    fn set_config_impl(&mut self, config: TransformerConfig<B::Input>) {
      self.config = config;
    }

    fn get_config_impl(&self) -> &TransformerConfig<B::Input> {
      &self.config
    }

    fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<B::Input> {
      &mut self.config
    }

    fn handle_error(&self, error: &StreamError<B::Input>) -> ErrorAction {
      match self.config.error_strategy {
        ErrorStrategy::Stop => ErrorAction::Stop,
        ErrorStrategy::Skip => ErrorAction::Skip,
        ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
        _ => ErrorAction::Stop,
      }
    }

    fn create_error_context(&self, item: Option<B::Input>) -> ErrorContext<B::Input> {
      ErrorContext {
        timestamp: chrono::Utc::now(),
        item,
        component_name: self.component_info().name,
        component_type: std::any::type_name::<Self>().to_string(),
      }
    }

    fn component_info(&self) -> ComponentInfo {
      ComponentInfo {
        name: self
          .config
          .name
          .clone()
          .unwrap_or_else(|| "inference_transformer".to_string()),
        type_name: std::any::type_name::<Self>().to_string(),
      }
    }
  }
}

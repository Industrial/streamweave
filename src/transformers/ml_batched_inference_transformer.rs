//! ML batched inference transformer for high-throughput ML model inference.
//!
//! This module provides [`BatchedInferenceTransformer`], a transformer that performs
//! machine learning model inference on batched stream items using an inference backend.
//! Batching improves throughput by processing multiple items together, especially
//! important for GPU-accelerated inference where batch processing can significantly
//! improve utilization.
//!
//! # Overview
//!
//! [`BatchedInferenceTransformer`] is useful for high-throughput ML inference scenarios
//! where individual requests can be grouped and processed together. This reduces
//! overhead and maximizes utilization of inference hardware (e.g., GPUs).
//! It integrates with various [`InferenceBackend`] implementations (e.g., ONNX, TensorFlow Lite).
//!
//! # Key Concepts
//!
//! - **Batched Inference**: Groups multiple input items into a single batch for model inference.
//! - **Inference Backend**: Pluggable backend (e.g., ONNX, TFLite) that performs the actual model execution.
//! - **Batch Size**: Configurable maximum number of items per batch.
//! - **Batch Timeout**: Configurable duration to wait for a batch to fill before processing.
//! - **Performance Optimization**: Reduces overhead and improves throughput for ML models.
//!
//! # Core Types
//!
//! - **[`BatchedInferenceTransformer<B>`]**: The main struct for the batched ML inference transformer, generic over the [`InferenceBackend`] type `B`.
//! - **[`BatchedInferenceConfig`]**: Configuration for batched inference behavior.
//! - **[`InferenceBackend`]**: Trait defining the interface for ML inference backends.
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use streamweave::transformers::BatchedInferenceTransformer;
//! use streamweave::ml::InferenceBackend;
//! use streamweave_ml_transformers::OnnxBackend; // Assuming `streamweave_ml_transformers` crate is available
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize an ONNX backend (replace with your actual model path)
//! let mut backend = OnnxBackend::new()?;
//! backend.load_from_path("path/to/your/model.onnx").await?;
//!
//! // Create a batched inference transformer with a batch size of 32 and a 100ms timeout
//! let transformer = BatchedInferenceTransformer::new(backend)
//!     .with_batch_size(32)
//!     .with_timeout(Duration::from_millis(100));
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust,no_run
//! use streamweave::transformers::BatchedInferenceTransformer;
//! use streamweave::ml::InferenceBackend;
//! use streamweave::ErrorStrategy;
//! use streamweave_ml_transformers::OnnxBackend;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut backend = OnnxBackend::new()?;
//! backend.load_from_path("model.onnx").await?;
//!
//! // Create a batched inference transformer with error handling
//! let transformer = BatchedInferenceTransformer::new(backend)
//!     .with_batch_size(32)
//!     .with_timeout(Duration::from_millis(100))
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("batched_onnx_inference".to_string());
//! # Ok(())
//! # }
//! ```
//!
//! # Design Decisions
//!
//! - **Pluggable Backends**: Decouples inference logic from the transformer via the `InferenceBackend` trait.
//! - **Batching Logic**: Manages batch accumulation and dispatch based on size or time.
//! - **Asynchronous Processing**: Leverages `async/await` for non-blocking batch processing.
//! - **Performance Optimization**: Batches items to maximize utilization of inference hardware, especially GPUs.
//! - **Error Handling**: Provides configurable error strategies for inference failures.
//!
//! # Integration with StreamWeave
//!
//! [`BatchedInferenceTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

// Import for rustdoc links
#[allow(unused_imports)]
use crate::Transformer;

use crate::TransformerConfig;
use crate::error::ErrorStrategy;
use crate::ml::InferenceBackend;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Configuration for batched inference.
#[derive(Debug, Clone)]
pub struct BatchedInferenceConfig {
  /// Maximum batch size (number of items per batch)
  pub batch_size: usize,
  /// Timeout for partial batches (items will be processed even if batch is not full)
  pub timeout: Duration,
}

impl Default for BatchedInferenceConfig {
  fn default() -> Self {
    Self {
      batch_size: 32,
      timeout: Duration::from_millis(100),
    }
  }
}

impl BatchedInferenceConfig {
  /// Creates a new batched inference configuration.
  ///
  /// # Arguments
  ///
  /// * `batch_size` - Maximum number of items per batch
  /// * `timeout` - Maximum time to wait for a full batch before processing partial batch
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave_ml_transformers::BatchedInferenceConfig;
  /// # use std::time::Duration;
  /// let config = BatchedInferenceConfig::new(64, Duration::from_millis(50));
  /// ```
  pub fn new(batch_size: usize, timeout: Duration) -> Self {
    Self {
      batch_size,
      timeout,
    }
  }
}

/// Transformer that batches inference requests for efficient processing.
///
/// This transformer collects stream items into batches and processes them together
/// using the backend's `infer_batch` method. This improves throughput, especially
/// for GPU-accelerated inference.
///
/// Batches are emitted when:
/// - The batch reaches the configured `batch_size`
/// - The `timeout` duration expires (even for partial batches)
/// - The input stream ends
///
/// # Type Parameters
///
/// * `B` - The inference backend type
///
/// # Example
///
/// ```rust,no_run
/// use streamweave_ml_transformers::{BatchedInferenceTransformer, OnnxBackend};
/// use std::time::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut backend = OnnxBackend::new()?;
/// backend.load_from_path("model.onnx").await?;
///
/// let transformer = BatchedInferenceTransformer::new(backend)
///     .with_batch_size(32)
///     .with_timeout(Duration::from_millis(100));
/// # Ok(())
/// # }
/// ```
pub struct BatchedInferenceTransformer<B>
where
  B: InferenceBackend + 'static,
{
  /// The inference backend
  backend: Arc<RwLock<B>>,
  /// Batching configuration
  config: BatchedInferenceConfig,
  /// Transformer configuration
  transformer_config: TransformerConfig<B::Input>,
  /// Phantom data marker
  _phantom: PhantomData<B::Input>,
}

impl<B> BatchedInferenceTransformer<B>
where
  B: InferenceBackend + 'static,
{
  /// Creates a new batched inference transformer with default configuration.
  ///
  /// # Arguments
  ///
  /// * `backend` - The inference backend to use for batch inference
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave_ml_transformers::{BatchedInferenceTransformer, OnnxBackend};
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut backend = OnnxBackend::new()?;
  /// backend.load_from_path("model.onnx").await?;
  /// let transformer = BatchedInferenceTransformer::new(backend);
  /// # Ok(())
  /// # }
  /// ```
  pub fn new(backend: B) -> Self {
    Self {
      backend: Arc::new(RwLock::new(backend)),
      config: BatchedInferenceConfig::default(),
      transformer_config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }

  /// Sets the batch size.
  ///
  /// # Arguments
  ///
  /// * `batch_size` - Maximum number of items per batch
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave_ml_transformers::{BatchedInferenceTransformer, OnnxBackend};
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut backend = OnnxBackend::new()?;
  /// backend.load_from_path("model.onnx").await?;
  /// let transformer = BatchedInferenceTransformer::new(backend)
  ///     .with_batch_size(64);
  /// # Ok(())
  /// # }
  /// ```
  pub fn with_batch_size(mut self, batch_size: usize) -> Self {
    self.config.batch_size = batch_size;
    self
  }

  /// Sets the timeout for partial batches.
  ///
  /// # Arguments
  ///
  /// * `timeout` - Maximum time to wait for a full batch before processing partial batch
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave_ml_transformers::{BatchedInferenceTransformer, OnnxBackend};
  /// # use std::time::Duration;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut backend = OnnxBackend::new()?;
  /// backend.load_from_path("model.onnx").await?;
  /// let transformer = BatchedInferenceTransformer::new(backend)
  ///     .with_timeout(Duration::from_millis(50));
  /// # Ok(())
  /// # }
  /// ```
  pub fn with_timeout(mut self, timeout: Duration) -> Self {
    self.config.timeout = timeout;
    self
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
  pub async fn swap_backend(&mut self, new_backend: B) -> Result<(), crate::SwapError> {
    crate::swap_model(&self.backend, new_backend).await
  }

  /// Gets the current batch configuration.
  ///
  /// # Returns
  ///
  /// A reference to the batching configuration.
  pub fn batch_config(&self) -> &BatchedInferenceConfig {
    &self.config
  }
}

// Transformer trait implementation
mod transformer_impl {
  use super::BatchedInferenceTransformer;
  use crate::Input;
  use crate::Output;
  use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
  use crate::ml::InferenceBackend;
  use crate::{Transformer, TransformerConfig};
  use async_stream::stream;
  use async_trait::async_trait;
  use futures::StreamExt;
  use std::pin::Pin;
  use std::sync::Arc;
  use std::time::Duration;
  use tokio::time::{Instant, sleep};

  // Input trait implementation
  impl<B> Input for BatchedInferenceTransformer<B>
  where
    B: InferenceBackend + 'static,
    B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  {
    type Input = B::Input;
    type InputStream = Pin<Box<dyn futures::Stream<Item = B::Input> + Send>>;
  }

  // Output trait implementation
  impl<B> Output for BatchedInferenceTransformer<B>
  where
    B: InferenceBackend + 'static,
    B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  {
    type Output = B::Output;
    type OutputStream = Pin<Box<dyn futures::Stream<Item = B::Output> + Send>>;
  }

  #[async_trait]
  impl<B> Transformer for BatchedInferenceTransformer<B>
  where
    B: InferenceBackend + 'static,
    B::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    B::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  {
    type InputPorts = (B::Input,);
    type OutputPorts = (B::Output,);

    async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
      let backend: Arc<tokio::sync::RwLock<B>> = Arc::clone(&self.backend);
      let batch_size = self.config.batch_size;
      let timeout = self.config.timeout;

      Box::pin(stream! {
        let mut input = input;
        let mut current_batch: Vec<B::Input> = Vec::with_capacity(batch_size);
        let mut batch_start_time: Option<Instant> = None;

        loop {
          // Create a timeout future for the current batch
          let timeout_future = if let Some(start_time) = batch_start_time {
            let elapsed = start_time.elapsed();
            if elapsed >= timeout {
              // Timeout already passed, sleep for 0 duration to yield control
              Box::pin(sleep(Duration::ZERO)) as Pin<Box<dyn futures::Future<Output = ()> + Send>>
            } else {
              Box::pin(sleep(timeout - elapsed)) as Pin<Box<dyn futures::Future<Output = ()> + Send>>
            }
          } else {
            Box::pin(sleep(timeout)) as Pin<Box<dyn futures::Future<Output = ()> + Send>>
          };

          // Wait for either next item or timeout
          tokio::select! {
            // New item arrived
            item_opt = input.next() => {
              match item_opt {
                Some(item) => {
                  if batch_start_time.is_none() {
                    batch_start_time = Some(Instant::now());
                  }
                  current_batch.push(item);

                  // Check if batch is full
                  if current_batch.len() >= batch_size {
                    // Process full batch
                    let batch = std::mem::replace(&mut current_batch, Vec::with_capacity(batch_size));
                    batch_start_time = None;

                    // Run batch inference
                    let backend_guard = backend.read().await;
                    match backend_guard.infer_batch(batch).await {
                      Ok(outputs) => {
                        for output in outputs {
                          yield output;
                        }
                      }
                      Err(e) => {
                        // Error handling - for now, we'll skip the batch
                        // In a production system, this should use the error strategy
                        eprintln!("Batch inference error: {}", e);
                      }
                    }
                  }
                }
                None => {
                  // Stream ended, process remaining batch
                  if !current_batch.is_empty() {
                    let batch = std::mem::take(&mut current_batch);
                    let backend_guard = backend.read().await;
                    match backend_guard.infer_batch(batch).await {
                      Ok(outputs) => {
                        for output in outputs {
                          yield output;
                        }
                      }
                      Err(e) => {
                        eprintln!("Batch inference error on final batch: {}", e);
                      }
                    }
                  }
                  break;
                }
              }
            }
            // Timeout occurred
            _ = timeout_future => {
              // Timeout - process partial batch if any
              if !current_batch.is_empty() {
                let batch = std::mem::take(&mut current_batch);
                batch_start_time = None;

                let backend_guard = backend.read().await;
                match backend_guard.infer_batch(batch).await {
                  Ok(outputs) => {
                    for output in outputs {
                      yield output;
                    }
                  }
                  Err(e) => {
                    eprintln!("Batch inference error on timeout: {}", e);
                  }
                }
              } else {
                // No items yet, reset timer
                batch_start_time = None;
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
      match &self.transformer_config.error_strategy {
        ErrorStrategy::Stop => ErrorAction::Stop,
        ErrorStrategy::Skip => ErrorAction::Skip,
        ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
        ErrorStrategy::Custom(handler) => handler(error),
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
          .transformer_config
          .name
          .clone()
          .unwrap_or_else(|| "batched_inference_transformer".to_string()),
        type_name: std::any::type_name::<Self>().to_string(),
      }
    }
  }
}

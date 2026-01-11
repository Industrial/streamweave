//! Batch transformer for grouping stream items into batches.
//!
//! This module provides [`BatchTransformer<T>`], a transformer that groups items
//! from a stream into batches of a specified size. It collects incoming items until
//! the batch size is reached, then emits them as a `Vec<T>`. This is useful for
//! batch processing, batching API calls, or grouping items for bulk operations.
//!
//! # Overview
//!
//! [`BatchTransformer`] collects stream items into fixed-size batches. When a batch
//! reaches the specified size, it is emitted as a `Vec<T>`. The final batch may be
//! smaller than the batch size if the stream ends before the batch is full.
//!
//! # Key Concepts
//!
//! - **Fixed Batch Size**: Groups items into batches of a specified size
//! - **Batch Emission**: Emits batches when size is reached or stream ends
//! - **Final Batch**: The last batch may be smaller if stream ends before batch is full
//! - **Type Transformation**: Changes output type from `T` to `Vec<T>`
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`BatchTransformer<T>`]**: Transformer that batches items into `Vec<T>`
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::BatchTransformer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that batches items into groups of 10
//! let transformer = BatchTransformer::<i32>::new(10)?;
//!
//! // Input: stream of individual items: 1, 2, 3, ..., 25
//! // Output: batches of 10 items: [1..10], [11..20], [21..25]
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::BatchTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = BatchTransformer::<String>::new(100)?
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("batch-processor".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Fixed Size Batches**: Uses fixed batch size for predictable batch sizes
//! - **Partial Final Batch**: Emits final batch even if incomplete (useful for processing)
//! - **Generic Type**: Generic over item type for maximum flexibility
//! - **Validation**: Requires batch size > 0 to prevent invalid configurations
//!
//! # Integration with StreamWeave
//!
//! [`BatchTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::marker::PhantomData;
use std::pin::Pin;

/// A transformer that groups items in a stream into batches of a specified size.
///
/// This transformer collects incoming items until the specified `size` is reached,
/// then emits them as a `Vec<T>`.
pub struct BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The number of items to include in each batch.
  pub size: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the input type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `BatchTransformer` with the given batch size.
  ///
  /// # Arguments
  ///
  /// * `size` - The number of items to include in each batch. Must be greater than zero.
  ///
  /// # Returns
  ///
  /// A `Result` containing the `BatchTransformer` if `size` is valid, or an error if `size` is zero.
  pub fn new(size: usize) -> Result<Self, Box<StreamError<T>>> {
    if size == 0 {
      return Err(Box::new(StreamError::new(
        Box::new(std::io::Error::new(
          std::io::ErrorKind::InvalidInput,
          "Batch size must be greater than zero",
        )),
        ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          component_name: "batch_transformer".to_string(),
          component_type: std::any::type_name::<Self>().to_string(),
        },
        ComponentInfo {
          name: "batch_transformer".to_string(),
          type_name: std::any::type_name::<Self>().to_string(),
        },
      )));
    }
    Ok(Self {
      size,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    })
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Input for BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

#[async_trait]
impl<T> Transformer for BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (Vec<T>,);

  async fn transform(&mut self, mut input: Self::InputStream) -> Self::OutputStream {
    let size = self.size;
    let mut current_batch: Vec<T> = Vec::with_capacity(size);

    Box::pin(async_stream::stream! {
      while let Some(item) = input.next().await {
        current_batch.push(item);
        if current_batch.len() == size {
          yield current_batch;
          current_batch = Vec::with_capacity(size);
        }
      }
      if !current_batch.is_empty() {
        yield current_batch;
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
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
        .unwrap_or_else(|| "batch_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

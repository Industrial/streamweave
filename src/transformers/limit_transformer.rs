//! Limit transformer for StreamWeave.
//!
//! This module provides [`LimitTransformer`], a transformer that limits the number
//! of items passed through a stream. It stops producing items after a specified
//! number of items have been processed, effectively truncating the stream.
//!
//! # Overview
//!
//! [`LimitTransformer`] is useful for truncating streams to a specific number of
//! items. This is commonly used for testing, sampling, or processing only a subset
//! of data from a larger stream.
//!
//! # Key Concepts
//!
//! - **Item Limiting**: Stops producing items after a specified count
//! - **Stream Truncation**: Effectively truncates the stream at the limit
//! - **Early Termination**: Stops processing once the limit is reached
//! - **Generic Type**: Works with any item type
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`LimitTransformer<T>`]**: Transformer that limits the number of items
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::LimitTransformer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that limits to 10 items
//! let transformer = LimitTransformer::new(10);
//!
//! // Input: [1, 2, 3, ..., 100]
//! // Output: [1, 2, 3, ..., 10]  (only first 10 items)
//! # Ok(())
//! # }
//! ```
//!
//! ## Sampling Data
//!
//! ```rust
//! use streamweave::transformers::LimitTransformer;
//!
//! // Sample first 100 items from a large stream
//! let transformer = LimitTransformer::new(100)
//!     .with_name("sampler".to_string());
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::LimitTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = LimitTransformer::new(50)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("limit-50".to_string());
//! ```
//!
//! # Behavior
//!
//! The transformer processes items sequentially and stops after emitting the
//! specified number of items. Items beyond the limit are not processed or emitted.
//!
//! # Design Decisions
//!
//! - **Simple Limit**: Uses a `usize` for straightforward count-based limiting
//! - **Early Termination**: Stops processing once limit is reached (efficient)
//! - **Generic Type**: Works with any item type for maximum flexibility
//! - **Stream Take**: Uses `StreamExt::take` for efficient stream truncation
//!
//! # Integration with StreamWeave
//!
//! [`LimitTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::marker::PhantomData;
use std::pin::Pin;

/// A transformer that limits the number of items passed through the stream.
///
/// This transformer stops producing items after a specified number of items
/// have been processed, effectively truncating the stream.
pub struct LimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The maximum number of items to allow through.
  pub limit: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> LimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `LimitTransformer` with the given limit.
  ///
  /// # Arguments
  ///
  /// * `limit` - The maximum number of items to allow through the stream.
  pub fn new(limit: usize) -> Self {
    Self {
      limit,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
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

impl<T> Input for LimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for LimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for LimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let limit = self.limit;
    Box::pin(input.take(limit))
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
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "limit_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

//! Delay transformer for adding delays between stream items.
//!
//! This module provides [`DelayTransformer<T>`], a transformer that adds a delay
//! before emitting each item in the stream. This is useful for rate limiting,
//! scheduled processing, retry logic with backoff, and controlling throughput
//! in streaming pipelines.
//!
//! # Overview
//!
//! [`DelayTransformer`] introduces a delay between items in a stream. Each item
//! waits for the specified duration before being emitted, effectively slowing down
//! the stream. This is useful for rate limiting, controlling API call frequency,
//! or implementing backoff strategies.
//!
//! # Key Concepts
//!
//! - **Fixed Delay**: Adds a constant delay before each item
//! - **Rate Limiting**: Useful for limiting throughput and API call rates
//! - **Async Delay**: Uses Tokio's async sleep for efficient delay handling
//! - **Stream Control**: Controls the rate of item emission
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`DelayTransformer<T>`]**: Transformer that delays items by a duration
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::DelayTransformer;
//! use streamweave::PipelineBuilder;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that delays each item by 1 second
//! let transformer = DelayTransformer::new(Duration::from_secs(1));
//!
//! // Each item will be delayed by 1 second before being emitted
//! # Ok(())
//! # }
//! ```
//!
//! ## Rate Limiting
//!
//! ```rust
//! use streamweave::transformers::DelayTransformer;
//! use std::time::Duration;
//!
//! // Create a transformer that limits to 10 items per second (100ms delay)
//! let transformer = DelayTransformer::new(Duration::from_millis(100));
//! ```
//!
//! # Design Decisions
//!
//! - **Fixed Delay**: Uses a fixed delay duration (not configurable per item)
//! - **Async Sleep**: Uses Tokio's async sleep for efficient, non-blocking delays
//! - **Between Items**: Delay occurs before emitting each item
//! - **Simple API**: Straightforward duration-based configuration
//!
//! # Integration with StreamWeave
//!
//! [`DelayTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// Transformer that delays items by a specified duration.
///
/// Adds a delay before emitting each item in the stream. Useful for rate limiting,
/// scheduled processing, and retry logic with exponential backoff.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::DelayTransformer;
/// use std::time::Duration;
///
/// let transformer = DelayTransformer::new(Duration::from_secs(1));
/// // Each item will be delayed by 1 second before being emitted
/// ```
pub struct DelayTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Duration to delay each item
  duration: std::time::Duration,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
}

impl<T> DelayTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `DelayTransformer` with the specified duration.
  ///
  /// # Arguments
  ///
  /// * `duration` - The duration to delay each item before emitting it.
  ///
  /// # Returns
  ///
  /// A new `DelayTransformer` instance.
  pub fn new(duration: std::time::Duration) -> Self {
    Self {
      duration,
      config: TransformerConfig::default(),
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

impl<T> Clone for DelayTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      duration: self.duration,
      config: self.config.clone(),
    }
  }
}

impl<T> Input for DelayTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for DelayTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for DelayTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let duration = self.duration;
    Box::pin(input.then(move |item| async move {
      tokio::time::sleep(duration).await;
      item
    }))
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
      ErrorStrategy::Custom(ref handler) => handler(error),
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
        .unwrap_or_else(|| "delay_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

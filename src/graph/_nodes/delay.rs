//! Delay node for adding delays between stream items.
//!
//! This module provides [`Delay`], a graph node that adds a delay before emitting
//! each item in the stream. It wraps [`DelayTransformer`] for use in StreamWeave graphs.
//! This is useful for rate limiting, scheduled processing, and retry logic with
//! exponential backoff.
//!
//! # Overview
//!
//! [`Delay`] is useful for controlling the rate of item emission in graph-based
//! pipelines. It introduces a fixed delay before each item is passed through,
//! making it ideal for rate limiting and time-based processing scenarios.
//!
//! # Key Concepts
//!
//! - **Fixed Delay**: Introduces a configurable delay before each item
//! - **Rate Limiting**: Useful for controlling throughput and avoiding
//!   overwhelming downstream systems
//! - **Scheduled Processing**: Enables time-based processing patterns
//! - **Transformer Wrapper**: Wraps `DelayTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`Delay<T>`]**: Node that delays items by a specified duration
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::Delay;
//! use std::time::Duration;
//!
//! // Create a delay node that delays each item by 1 second
//! let delay = Delay::<i32>::new(Duration::from_secs(1));
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::Delay;
//! use streamweave::ErrorStrategy;
//! use std::time::Duration;
//!
//! // Create a delay node with error handling
//! let delay = Delay::<String>::new(Duration::from_millis(100))
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("rate-limiter".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Fixed Duration**: Uses a fixed duration delay for simplicity and
//!   predictability
//! - **Async Sleep**: Uses Tokio's async sleep for non-blocking delays
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`Delay`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::DelayTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that delays items by a specified duration.
///
/// This node wraps `DelayTransformer` for use in graphs. It adds a delay before
/// emitting each item in the stream.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Delay, TransformerNode};
/// use std::time::Duration;
///
/// let delay = Delay::new(Duration::from_secs(1));
/// let node = TransformerNode::from_transformer(
///     "delay".to_string(),
///     delay,
/// );
/// ```
pub struct Delay<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying delay transformer
  transformer: DelayTransformer<T>,
}

impl<T> Delay<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Delay` node with the specified duration.
  ///
  /// # Arguments
  ///
  /// * `duration` - The duration to delay each item before emitting it.
  ///
  /// # Returns
  ///
  /// A new `Delay` node instance.
  pub fn new(duration: std::time::Duration) -> Self {
    Self {
      transformer: DelayTransformer::new(duration),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
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

impl<T> Clone for Delay<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for Delay<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Delay<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Delay<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}

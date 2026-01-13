//! # Rate Limit Node
//!
//! This module provides [`RateLimit`], a graph node that rate-limits items passing
//! through a stream. It wraps [`RateLimitTransformer`] for use in StreamWeave graphs.
//! It ensures that only a specified number of items are processed within a given time
//! window, preventing overload and ensuring fair resource usage.
//!
//! # Overview
//!
//! [`RateLimit`] is useful for controlling the throughput of data processing in
//! graph-based pipelines. It limits the number of items that can pass through
//! within a configurable time window, making it ideal for rate-limiting API calls,
//! database queries, or any operation that needs throughput control.
//!
//! # Key Concepts
//!
//! - **Rate Limiting**: Limits the number of items processed within a time window
//! - **Time Window**: Configurable sliding window for rate limit calculations
//! - **Item Filtering**: Items exceeding the rate limit are filtered out (dropped)
//! - **Thread-Safe**: Uses atomic counters and locks for concurrent access
//! - **Transformer Wrapper**: Wraps `RateLimitTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`RateLimit<T>`]**: Node that rate-limits items of type `T`
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::RateLimit;
//! use tokio::time::Duration;
//!
//! // Create a rate limit node: max 100 items per second
//! let rate_limit = RateLimit::new(100, Duration::from_secs(1));
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::RateLimit;
//! use streamweave::ErrorStrategy;
//! use tokio::time::Duration;
//!
//! // Create a rate limit node with error handling strategy
//! let rate_limit = RateLimit::new(100, Duration::from_secs(1))
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("api-rate-limit".to_string());
//! ```
//!
//! ## Different Rate Limits
//!
//! ```rust
//! use streamweave::graph::nodes::RateLimit;
//! use tokio::time::Duration;
//!
//! // 10 items per second
//! let rate_limit = RateLimit::new(10, Duration::from_secs(1));
//!
//! // 1000 items per minute
//! let rate_limit = RateLimit::new(1000, Duration::from_secs(60));
//!
//! // 5 items per 100 milliseconds
//! let rate_limit = RateLimit::new(5, Duration::from_millis(100));
//! ```
//!
//! # Design Decisions
//!
//! - **Sliding Window**: Uses a sliding time window for accurate rate limiting
//! - **Atomic Counters**: Uses atomic operations for thread-safe counting
//! - **Filtering**: Items exceeding the rate limit are filtered out (not retried)
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`RateLimit`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::RateLimitTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::time::Duration;

/// Node that rate-limits items passing through.
///
/// This node wraps `RateLimitTransformer` for use in graphs. It ensures that
/// only a specified number of items are processed within a given time window.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{RateLimit, TransformerNode};
/// use tokio::time::Duration;
///
/// let rate_limit = RateLimit::new(100, Duration::from_secs(1));
/// let node = TransformerNode::from_transformer(
///     "rate_limit".to_string(),
///     rate_limit,
/// );
/// ```
pub struct RateLimit<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying rate limit transformer
  transformer: RateLimitTransformer<T>,
}

impl<T> RateLimit<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `RateLimit` node with the specified rate limit and time window.
  ///
  /// # Arguments
  ///
  /// * `rate_limit` - The maximum number of items allowed within the time window.
  /// * `time_window` - The time window for rate limiting.
  pub fn new(rate_limit: usize, time_window: Duration) -> Self {
    Self {
      transformer: RateLimitTransformer::new(rate_limit, time_window),
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

impl<T> Clone for RateLimit<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: RateLimitTransformer::new(
        self.transformer.rate_limit,
        self.transformer.time_window,
      ),
    }
    .with_error_strategy(self.transformer.config.error_strategy.clone())
    .with_name(
      self
        .transformer
        .config
        .name
        .clone()
        .unwrap_or_else(|| "rate_limit".to_string()),
    )
  }
}

impl<T> Input for RateLimit<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for RateLimit<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for RateLimit<T>
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

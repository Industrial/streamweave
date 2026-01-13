//! # Retry Node
//!
//! Graph node for implementing retry logic with configurable backoff in StreamWeave graphs.
//!
//! This module provides [`Retry`], a graph node that automatically retries failed
//! operations with configurable backoff delays. It wraps [`RetryTransformer`] for use
//! in StreamWeave graphs, enabling resilience patterns for handling transient failures.
//!
//! # Overview
//!
//! [`Retry`] is useful for implementing retry logic in graph-based pipelines. It
//! automatically retries failed operations up to a maximum number of times with
//! configurable backoff delays between attempts. This enables pipelines to handle
//! transient failures gracefully without manual intervention.
//!
//! # Key Concepts
//!
//! - **Automatic Retries**: Automatically retries failed operations without manual intervention
//! - **Configurable Backoff**: Configurable delay between retry attempts
//! - **Maximum Retries**: Limits the number of retry attempts to prevent infinite loops
//! - **Transient Failure Handling**: Designed for handling transient failures (network issues, temporary service unavailability)
//! - **Transformer Wrapper**: Wraps `RetryTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`Retry<T>`]**: Node that retries failed operations with configurable backoff
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::Retry;
//! use tokio::time::Duration;
//!
//! // Create a retry node that retries up to 3 times with 1 second backoff
//! let retry = Retry::<i32>::new(3, Duration::from_secs(1));
//! ```
//!
//! ## Custom Backoff Duration
//!
//! ```rust
//! use streamweave::graph::nodes::Retry;
//! use tokio::time::Duration;
//!
//! // Create a retry node with longer backoff for slow services
//! let retry = Retry::<String>::new(5, Duration::from_secs(5));
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::Retry;
//! use streamweave::ErrorStrategy;
//! use tokio::time::Duration;
//!
//! // Create a retry node with error handling strategy
//! let retry = Retry::<i32>::new(3, Duration::from_secs(1))
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("api-retry".to_string());
//! ```
//!
//! # Design Decisions
//!
//! ## Retry Strategy
//!
//! Uses fixed backoff duration between retries. This provides predictable retry
//! behavior and is suitable for most use cases. For more complex backoff strategies
//! (exponential, jittered), consider using specialized retry transformers.
//!
//! ## Maximum Retries
//!
//! Limits the number of retry attempts to prevent infinite retry loops and resource
//! exhaustion. After the maximum number of retries is reached, the operation is
//! considered permanently failed.
//!
//! ## Transient Failure Focus
//!
//! Designed for handling transient failures (network issues, temporary service
//! unavailability). For permanent failures or validation errors, consider using
//! different error handling strategies.
//!
//! ## Transformer Wrapper
//!
//! Wraps existing transformer for consistency with other graph nodes. This allows
//! retry logic to be easily composed into graph-based pipelines.
//!
//! # Integration with StreamWeave
//!
//! [`Retry`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].
//!
//! # Common Patterns
//!
//! ## Network Operation Retries
//!
//! Retry network operations that may fail due to transient issues:
//!
//! ```rust
//! use streamweave::graph::nodes::Retry;
//! use tokio::time::Duration;
//!
//! // Retry HTTP requests with 3 attempts and 2 second backoff
//! let retry = Retry::<String>::new(3, Duration::from_secs(2));
//! ```
//!
//! ## Database Operation Retries
//!
//! Retry database operations that may fail due to connection issues:
//!
//! ```rust
//! use streamweave::graph::nodes::Retry;
//! use tokio::time::Duration;
//!
//! // Retry database queries with 5 attempts and 1 second backoff
//! let retry = Retry::<serde_json::Value>::new(5, Duration::from_secs(1));
//! ```
//!
//! ## Combining with Circuit Breaker
//!
//! Use retry logic in combination with circuit breakers for comprehensive resilience:
//!
//! ```rust
//! use streamweave::graph::nodes::{Retry, CircuitBreaker};
//! use tokio::time::Duration;
//!
//! // First retry, then circuit breaker
//! let retry = Retry::<i32>::new(3, Duration::from_secs(1));
//! let circuit_breaker = CircuitBreaker::<i32>::new(5, Duration::from_secs(10));
//! // Chain: retry -> circuit_breaker
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::RetryTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::time::Duration;

/// Node that retries failed operations with configurable retry logic.
///
/// This node wraps `RetryTransformer` for use in graphs. It automatically
/// retries failed operations up to a maximum number of times with configurable
/// backoff delay.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Retry, TransformerNode};
/// use tokio::time::Duration;
///
/// let retry = Retry::new(3, Duration::from_secs(1));
/// let node = TransformerNode::from_transformer(
///     "retry".to_string(),
///     retry,
/// );
/// ```
pub struct Retry<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying retry transformer
  transformer: RetryTransformer<T>,
}

impl<T> Retry<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Retry` node with the specified maximum retries and backoff duration.
  ///
  /// # Arguments
  ///
  /// * `max_retries` - The maximum number of retry attempts.
  /// * `backoff` - The backoff duration between retry attempts.
  pub fn new(max_retries: usize, backoff: Duration) -> Self {
    Self {
      transformer: RetryTransformer::new(max_retries, backoff),
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

impl<T> Clone for Retry<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for Retry<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Retry<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Retry<T>
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

//! Circuit breaker node for implementing resilience patterns.
//!
//! This module provides [`CircuitBreaker`], a graph node that implements the circuit
//! breaker pattern for resilience. It monitors failures and opens the circuit (stops
//! processing) when the failure threshold is exceeded, automatically resetting after
//! a timeout. It wraps [`CircuitBreakerTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`CircuitBreaker`] is useful for protecting downstream services from cascading
//! failures. It monitors the failure rate and automatically stops forwarding requests
//! when failures exceed a threshold, allowing the downstream service to recover
//! before retrying.
//!
//! # Key Concepts
//!
//! - **Circuit States**: The circuit can be in Closed (normal operation), Open
//!   (stopping requests), or Half-Open (testing recovery) states
//! - **Failure Threshold**: The number of failures that trigger circuit opening
//! - **Reset Timeout**: The duration to wait before attempting to reset the circuit
//! - **Resilience Pattern**: Prevents cascading failures by stopping requests when
//!   downstream services are failing
//!
//! # Core Types
//!
//! - **[`CircuitBreaker<T>`]**: Node that implements the circuit breaker pattern
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::CircuitBreaker;
//! use tokio::time::Duration;
//!
//! // Create a circuit breaker that opens after 5 failures
//! // and resets after 10 seconds
//! let circuit_breaker = CircuitBreaker::<i32>::new(
//!     5,                              // failure threshold
//!     Duration::from_secs(10)        // reset timeout
//! );
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::CircuitBreaker;
//! use streamweave::ErrorStrategy;
//! use tokio::time::Duration;
//!
//! // Create a circuit breaker with error handling
//! let circuit_breaker = CircuitBreaker::<String>::new(3, Duration::from_secs(5))
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("api-circuit-breaker".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Circuit Breaker Pattern**: Implements the standard circuit breaker pattern
//!   for resilience
//! - **Failure Monitoring**: Tracks failure rates and automatically opens circuit
//!   when threshold is exceeded
//! - **Automatic Recovery**: Attempts to reset circuit after timeout period
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`CircuitBreaker`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::CircuitBreakerTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::time::Duration;

/// Node that implements the circuit breaker pattern for resilience.
///
/// This node wraps `CircuitBreakerTransformer` for use in graphs. It monitors
/// failures and opens the circuit when the failure threshold is exceeded,
/// automatically resetting after a timeout.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{CircuitBreaker, TransformerNode};
/// use tokio::time::Duration;
///
/// let circuit_breaker = CircuitBreaker::new(5, Duration::from_secs(10));
/// let node = TransformerNode::from_transformer(
///     "circuit_breaker".to_string(),
///     circuit_breaker,
/// );
/// ```
pub struct CircuitBreaker<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying circuit breaker transformer
  transformer: CircuitBreakerTransformer<T>,
}

impl<T> CircuitBreaker<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `CircuitBreaker` node with the specified failure threshold and reset timeout.
  ///
  /// # Arguments
  ///
  /// * `failure_threshold` - The number of failures before the circuit opens.
  /// * `reset_timeout` - The duration to wait before attempting to reset the circuit.
  pub fn new(failure_threshold: usize, reset_timeout: Duration) -> Self {
    Self {
      transformer: CircuitBreakerTransformer::new(failure_threshold, reset_timeout),
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

impl<T> Clone for CircuitBreaker<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for CircuitBreaker<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for CircuitBreaker<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for CircuitBreaker<T>
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

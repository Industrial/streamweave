//! Timeout transformer for timing out items after a specified duration.
//!
//! This module provides [`Timeout`] and [`TimeoutError`], types for timing out
//! items after a specified duration in graph-based pipelines. It wraps items in
//! `Result<T, TimeoutError>`, returning `Ok(item)` if the item is processed
//! within the timeout, or `Err(TimeoutError)` if it times out. It implements
//! the [`Transformer`] trait for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`Timeout`] is useful for adding timeout handling to items in graph-based
//! pipelines. It processes items and applies a timeout, making it ideal for
//! preventing long-running operations from blocking the pipeline indefinitely.
//!
//! # Key Concepts
//!
//! - **Timeout Handling**: Applies a timeout duration to items
//! - **Result Wrapping**: Wraps items in `Result<T, TimeoutError>` for error handling
//! - **Duration-Based**: Uses `Duration` for flexible timeout specification
//! - **Transformer Trait**: Implements `Transformer` for graph integration
//!
//! # Core Types
//!
//! - **[`Timeout<T>`]**: Transformer that times out items after a specified duration
//! - **[`TimeoutError`]**: Error type for timeout operations
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::Timeout;
//! use std::time::Duration;
//!
//! // Create a timeout transformer with 5 second timeout
//! let timeout = Timeout::<i32>::new(Duration::from_secs(5));
//! ```
//!
//! ## With Different Durations
//!
//! ```rust
//! use streamweave::graph::nodes::Timeout;
//! use std::time::Duration;
//!
//! // Timeout with millisecond precision
//! let timeout = Timeout::<String>::new(Duration::from_millis(100));
//! ```
//!
//! # Design Decisions
//!
//! - **Result Wrapping**: Wraps items in `Result` for explicit timeout error handling
//! - **Duration-Based**: Uses `Duration` for flexible timeout specification
//! - **Transformer Trait**: Implements `Transformer` for integration with
//!   graph system
//!
//! # Integration with StreamWeave
//!
//! [`Timeout`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It applies timeouts to items, enabling timeout handling
//! patterns.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::marker::PhantomData;
use std::pin::Pin;

/// Error type for timeout operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeoutError;

impl std::fmt::Display for TimeoutError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Operation timed out")
  }
}

impl std::error::Error for TimeoutError {}

/// Transformer that times out items after a specified duration.
///
/// Wraps items in `Result<T, TimeoutError>`, returning `Ok(item)` if the item
/// is processed within the timeout, or `Err(TimeoutError)` if it times out.
///
/// # Example
///
/// ```rust
/// use crate::graph::control_flow::Timeout;
/// use crate::graph::node::TransformerNode;
/// use std::time::Duration;
///
/// let timeout = Timeout::new(Duration::from_secs(5));
/// let node = TransformerNode::from_transformer(
///     "timeout".to_string(),
///     timeout,
/// );
/// ```
pub struct Timeout<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// Timeout duration
  timeout: std::time::Duration,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
  /// Phantom data for type parameter
  _phantom: PhantomData<T>,
}

impl<T> Timeout<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Timeout` transformer with the specified timeout duration.
  ///
  /// # Arguments
  ///
  /// * `timeout` - The duration after which items will timeout.
  ///
  /// # Returns
  ///
  /// A new `Timeout` transformer instance.
  pub fn new(timeout: std::time::Duration) -> Self {
    Self {
      timeout,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }
}

impl<T> Input for Timeout<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Timeout<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Result<T, TimeoutError>;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<T, TimeoutError>> + Send>>;
}

#[async_trait]
impl<T> Transformer for Timeout<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (Result<T, TimeoutError>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let timeout = self.timeout;
    Box::pin(input.then(move |item| async move {
      tokio::time::timeout(timeout, async { item })
        .await
        .map_err(|_| TimeoutError)
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config.error_strategy() {
      crate::error::ErrorStrategy::Stop => ErrorAction::Stop,
      crate::error::ErrorStrategy::Skip => ErrorAction::Skip,
      crate::error::ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      crate::error::ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
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
        .name()
        .clone()
        .unwrap_or_else(|| "timeout".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

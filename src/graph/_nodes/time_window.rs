//! # Time Window Node
//!
//! Graph node that creates time-based windows of items. This module provides
//! [`TimeWindow`], a graph node that groups consecutive items into windows
//! based on a time duration, producing vectors of items as windows are created.
//! It wraps [`TimeWindowTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`TimeWindow`] is useful for creating time-based batches in graph-based
//! pipelines. It collects items that arrive within a specified time duration
//! and emits them as vectors when the time window expires, making it ideal
//! for time-based aggregation and batch processing.
//!
//! # Key Concepts
//!
//! - **Time-Based Windowing**: Groups items based on time duration rather than count
//! - **Duration Configuration**: Uses `Duration` for flexible time window specification
//! - **Vector Output**: Produces `Vec<T>` instead of `T` (one vector per time window)
//! - **Time-Based Batching**: Useful for processing items in time-based batches
//! - **Transformer Wrapper**: Wraps `TimeWindowTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`TimeWindow<T>`]**: Node that creates time-based windows of items
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::TimeWindow;
//! use std::time::Duration;
//!
//! // Create time windows of 5 seconds
//! let time_window = TimeWindow::<i32>::new(Duration::from_secs(5));
//! ```
//!
//! ## Different Durations
//!
//! ```rust
//! use streamweave::graph::nodes::TimeWindow;
//! use std::time::Duration;
//!
//! // Short windows for frequent processing (1 second)
//! let short_window = TimeWindow::<String>::new(Duration::from_secs(1));
//!
//! // Long windows for batch processing (1 minute)
//! let long_window = TimeWindow::<String>::new(Duration::from_secs(60));
//!
//! // Millisecond precision
//! let ms_window = TimeWindow::<i32>::new(Duration::from_millis(500));
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::TimeWindow;
//! use streamweave::ErrorStrategy;
//! use std::time::Duration;
//!
//! // Create a time window node with error handling
//! let time_window = TimeWindow::<i32>::new(Duration::from_secs(10))
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("time-batcher".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Duration-Based**: Uses `Duration` for flexible time window specification
//! - **Time-Based Windowing**: Windows are based on elapsed time, not item count
//! - **Vector Output**: Produces `Vec<T>` to represent time-based batches
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`TimeWindow`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`]. Note that the output
//! type is `Vec<T>` rather than `T`, representing a batch of items collected
//! within the time window.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::TimeWindowTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::time::Duration;

/// Node that creates time-based windows of items.
///
/// This node wraps `TimeWindowTransformer` for use in graphs. It groups
/// consecutive items into windows based on time duration, producing vectors
/// of items as windows are created.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{TimeWindow, TransformerNode};
/// use tokio::time::Duration;
///
/// let time_window = TimeWindow::new(Duration::from_secs(5));
/// let node = TransformerNode::from_transformer(
///     "time_window".to_string(),
///     time_window,
/// );
/// ```
pub struct TimeWindow<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying time window transformer
  transformer: TimeWindowTransformer<T>,
}

impl<T> TimeWindow<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `TimeWindow` node with the specified window duration.
  ///
  /// # Arguments
  ///
  /// * `duration` - The duration of each window.
  pub fn new(duration: Duration) -> Self {
    Self {
      transformer: TimeWindowTransformer::new(duration),
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

impl<T> Clone for TimeWindow<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for TimeWindow<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for TimeWindow<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

#[async_trait]
impl<T> Transformer for TimeWindow<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (Vec<T>,);

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

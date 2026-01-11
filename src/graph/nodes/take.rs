//! Take node for taking a limited number of items from streams.
//!
//! This module provides [`Take`], a graph node that takes a specified number
//! of items from the beginning of a stream, then stops producing items. It
//! wraps [`TakeTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`Take`] is useful for limiting processing to a fixed number of items in
//! graph-based pipelines. It passes through only the first N items and then
//! stops producing items, making it ideal for testing, sampling, or processing
//! a fixed number of items.
//!
//! # Key Concepts
//!
//! - **Item Taking**: Takes a specified number of items from the start
//! - **Fixed Count**: Uses a fixed count to determine how many items to take
//! - **Early Termination**: Stops producing items after the count is reached
//! - **Transformer Wrapper**: Wraps `TakeTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`Take<T>`]**: Node that takes a limited number of items from a stream
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::Take;
//!
//! // Take only the first 100 items
//! let take = Take::<i32>::new(100);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::Take;
//! use streamweave::ErrorStrategy;
//!
//! // Create a take node with error handling
//! let take = Take::<String>::new(50)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("sample-taker".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Fixed Count Take**: Uses a fixed count for simplicity and predictability
//! - **Early Termination**: Stops processing after count is reached for efficiency
//! - **Stream-Based**: Works with async streams for efficient processing
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`Take`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::TakeTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that takes the first N items from the stream.
///
/// This node wraps `TakeTransformer` for use in graphs. It passes through only
/// the first `take` items from the input stream and then stops producing items.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Take, TransformerNode};
///
/// let take = Take::new(100);
/// let node = TransformerNode::from_transformer(
///     "take".to_string(),
///     take,
/// );
/// ```
pub struct Take<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying take transformer
  transformer: TakeTransformer<T>,
}

impl<T> Take<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Take` node with the specified take count.
  ///
  /// # Arguments
  ///
  /// * `take` - The number of items to take from the stream.
  pub fn new(take: usize) -> Self {
    Self {
      transformer: TakeTransformer::new(take),
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

impl<T> Clone for Take<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for Take<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Take<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Take<T>
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

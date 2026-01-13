//! Split-at node for splitting streams at a specified index.
//!
//! This module provides [`SplitAt`], a graph node that splits items at a
//! specified index. It collects items from the input stream and splits them
//! into two groups: items before the index and items at or after the index.
//! It wraps [`SplitAtTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`SplitAt`] is useful for splitting streams at a specific position in
//! graph-based pipelines. It collects all items, splits them at the specified
//! index, and outputs a tuple containing two vectors: items before the index
//! and items at or after the index.
//!
//! # Key Concepts
//!
//! - **Index-Based Splitting**: Splits at a specific index position
//! - **Two-Group Output**: Produces a tuple of two vectors (before and after)
//! - **Full Collection**: Collects all items before splitting
//! - **Transformer Wrapper**: Wraps `SplitAtTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`SplitAt<T>`]**: Node that splits items at a specified index
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::SplitAt;
//!
//! // Split stream at index 5 (first 5 items in first vector, rest in second)
//! let split_at = SplitAt::<i32>::new(5);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::SplitAt;
//! use streamweave::ErrorStrategy;
//!
//! // Create a split-at node with error handling
//! let split_at = SplitAt::<String>::new(10)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("index-splitter".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Index-Based**: Uses a fixed index for predictable splitting behavior
//! - **Full Collection**: Collects all items before splitting, which requires
//!   memory proportional to stream size
//! - **Tuple Output**: Outputs `(Vec<T>, Vec<T>)` to represent the two groups
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`SplitAt`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`]. Note that the
//! output type is `(Vec<T>, Vec<T>)` rather than `T`.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::SplitAtTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that splits items at a specified index.
///
/// This node wraps `SplitAtTransformer` for use in graphs. It collects items
/// from the input stream and splits them into two groups: items before the
/// index and items at or after the index.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{SplitAt, TransformerNode};
///
/// let split_at = SplitAt::new(5);
/// let node = TransformerNode::from_transformer(
///     "split_at".to_string(),
///     split_at,
/// );
/// ```
pub struct SplitAt<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying split-at transformer
  transformer: SplitAtTransformer<T>,
}

impl<T> SplitAt<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `SplitAt` node with the specified index.
  ///
  /// # Arguments
  ///
  /// * `index` - The index at which to split the stream.
  pub fn new(index: usize) -> Self {
    Self {
      transformer: SplitAtTransformer::new(index),
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

impl<T> Clone for SplitAt<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for SplitAt<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for SplitAt<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = (Vec<T>, Vec<T>);
  type OutputStream = Pin<Box<dyn Stream<Item = (Vec<T>, Vec<T>)> + Send>>;
}

#[async_trait]
impl<T> Transformer for SplitAt<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = ((Vec<T>, Vec<T>),);

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

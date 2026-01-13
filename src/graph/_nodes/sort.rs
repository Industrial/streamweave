//! Sort node for sorting items in streams.
//!
//! This module provides [`Sort`], a graph node that sorts items in a stream.
//! It collects all items from the input stream, sorts them, and emits them
//! in sorted order. It wraps [`SortTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`Sort`] is useful for sorting items in graph-based pipelines. It collects
//! all items, sorts them according to their natural ordering, and then emits
//! them in sorted order. This is essential for ordered processing and data
//! organization patterns.
//!
//! # Key Concepts
//!
//! - **Full Collection**: Collects all items before sorting (requires items to
//!   be `Ord`)
//! - **Natural Ordering**: Uses the type's natural ordering via `Ord` trait
//! - **Sorted Output**: Emits items in sorted order after collection
//! - **Transformer Wrapper**: Wraps `SortTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`Sort<T>`]**: Node that sorts items in a stream (requires `T: Ord`)
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::Sort;
//!
//! // Sort integers in ascending order
//! let sort = Sort::<i32>::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::Sort;
//! use streamweave::ErrorStrategy;
//!
//! // Create a sort node with error handling
//! let sort = Sort::<String>::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("string-sorter".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Full Collection**: Collects all items before sorting, which requires
//!   memory proportional to stream size
//! - **Natural Ordering**: Uses `Ord` trait for type-safe sorting
//! - **Stream-Based**: Works with async streams but requires full collection
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`Sort`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`]. Note that items
//! must implement `Ord` for sorting.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::SortTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that sorts items in the stream.
///
/// This node wraps `SortTransformer` for use in graphs. It collects all items
/// from the input stream, sorts them, and then emits them in sorted order.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Sort, TransformerNode};
///
/// let sort = Sort::new();
/// let node = TransformerNode::from_transformer(
///     "sort".to_string(),
///     sort,
/// );
/// ```
pub struct Sort<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  /// The underlying sort transformer
  transformer: SortTransformer<T>,
}

impl<T> Sort<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  /// Creates a new `Sort` node.
  pub fn new() -> Self {
    Self {
      transformer: SortTransformer::new(),
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

impl<T> Default for Sort<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for Sort<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for Sort<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Sort<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Sort<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
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

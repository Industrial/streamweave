//! Skip node for skipping items at the beginning of streams.
//!
//! This module provides [`Skip`], a graph node that skips a specified number
//! of items from the beginning of a stream, then passes through all subsequent
//! items. It wraps [`SkipTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`Skip`] is useful for skipping items at the beginning of graph-based
//! pipelines. It discards the first N items and then passes through all
//! subsequent items, making it ideal for skipping headers, metadata, or
//! initial items that should be ignored.
//!
//! # Key Concepts
//!
//! - **Item Skipping**: Skips a specified number of items from the start
//! - **Fixed Count**: Uses a fixed count to determine how many items to skip
//! - **Pass-Through**: After skipping, all subsequent items pass through unchanged
//! - **Transformer Wrapper**: Wraps `SkipTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`Skip<T>`]**: Node that skips items from the beginning of a stream
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::Skip;
//!
//! // Skip the first 10 items
//! let skip = Skip::<i32>::new(10);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::Skip;
//! use streamweave::ErrorStrategy;
//!
//! // Create a skip node with error handling
//! let skip = Skip::<String>::new(5)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("header-skipper".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Fixed Count Skip**: Uses a fixed count for simplicity and predictability
//! - **Stream-Based**: Works with async streams for efficient processing
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`Skip`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::SkipTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that skips a specified number of items from the stream.
///
/// This node wraps `SkipTransformer` for use in graphs. It discards the first
/// `skip` items and then passes through all subsequent items.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Skip, TransformerNode};
///
/// let skip = Skip::new(10);
/// let node = TransformerNode::from_transformer(
///     "skip".to_string(),
///     skip,
/// );
/// ```
pub struct Skip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying skip transformer
  transformer: SkipTransformer<T>,
}

impl<T> Skip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Skip` node with the specified skip count.
  ///
  /// # Arguments
  ///
  /// * `skip` - The number of items to skip from the beginning of the stream.
  pub fn new(skip: usize) -> Self {
    Self {
      transformer: SkipTransformer::new(skip),
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

impl<T> Clone for Skip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for Skip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Skip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Skip<T>
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

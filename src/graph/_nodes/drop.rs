//! Drop node for removing items from streams based on predicates.
//!
//! This module provides [`Drop`], a graph node that drops items from the stream
//! based on a predicate function. It wraps [`DropTransformer`] for use in
//! StreamWeave graphs. This is the inverse of [`crate::graph::nodes::Filter`] - items where the
//! predicate returns `true` are dropped (not passed through).
//!
//! # Overview
//!
//! [`Drop`] is useful for removing unwanted items from streams in graph-based
//! pipelines. It filters out items that match a condition, allowing only items
//! that don't match to pass through.
//!
//! # Key Concepts
//!
//! - **Predicate-Based Filtering**: Uses a predicate function to determine which
//!   items to drop
//! - **Inverse of Filter**: Drops items where predicate returns `true`, keeping
//!   items where predicate returns `false`
//! - **Selective Removal**: Useful for removing specific items based on conditions
//! - **Transformer Wrapper**: Wraps `DropTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`Drop<F, T>`]**: Node that drops items based on a predicate function
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::Drop;
//!
//! // Drop all items greater than 10
//! let drop = Drop::new(|x: &i32| *x > 10);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::Drop;
//! use streamweave::ErrorStrategy;
//!
//! // Drop empty strings with error handling
//! let drop = Drop::new(|s: &String| s.is_empty())
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("drop-empty".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Predicate Function**: Uses a closure for flexible item filtering
//! - **Inverse Logic**: Provides an alternative to `Filter` for cases where
//!   dropping is more intuitive than keeping
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`Drop`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

// Import for rustdoc links
#[allow(unused_imports)]
use super::filter::Filter;

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::DropTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that drops items from the stream based on a predicate function.
///
/// This node wraps `DropTransformer` for use in graphs. Items for which the
/// predicate returns `true` are dropped (not passed through), while items where
/// the predicate returns `false` are kept.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Drop, TransformerNode};
///
/// // Drop all items greater than 10
/// let drop = Drop::new(|x: &i32| *x > 10);
/// let node = TransformerNode::from_transformer(
///     "drop".to_string(),
///     drop,
/// );
/// ```
pub struct Drop<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying drop transformer
  transformer: DropTransformer<F, T>,
}

impl<F, T> Drop<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Drop` node with the given predicate function.
  ///
  /// Items for which the predicate returns `true` will be dropped.
  ///
  /// # Arguments
  ///
  /// * `predicate` - The function to use for determining which items to drop.
  pub fn new(predicate: F) -> Self {
    Self {
      transformer: DropTransformer::new(predicate),
    }
  }

  /// Sets the error strategy for this node.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl<F, T> Clone for Drop<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<F, T> Input for Drop<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<F, T> Output for Drop<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<F, T> Transformer for Drop<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
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

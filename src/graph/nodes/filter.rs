//! Filter node for filtering items in streams based on predicates.
//!
//! This module provides [`Filter`], a graph node that filters items in a stream
//! based on a predicate function. It wraps [`FilterTransformer`] for use in
//! StreamWeave graphs. Only items for which the predicate returns `true` are
//! passed through to the output.
//!
//! # Overview
//!
//! [`Filter`] is useful for selectively processing items in graph-based pipelines.
//! It allows only items that match a condition to pass through, filtering out
//! items that don't match. This is essential for data selection and conditional
//! processing patterns.
//!
//! # Key Concepts
//!
//! - **Predicate-Based Filtering**: Uses a predicate function to determine which
//!   items to keep
//! - **Selective Processing**: Only items where predicate returns `true` pass through
//! - **Data Selection**: Useful for selecting specific items based on conditions
//! - **Transformer Wrapper**: Wraps `FilterTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`Filter<F, T>`]**: Node that filters items based on a predicate function
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::Filter;
//!
//! // Filter to keep only items greater than 10
//! let filter = Filter::new(|x: &i32| *x > 10);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::Filter;
//! use streamweave::ErrorStrategy;
//!
//! // Filter non-empty strings with error handling
//! let filter = Filter::new(|s: &String| !s.is_empty())
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("non-empty-filter".to_string());
//! ```
//!
//! ## Complex Predicates
//!
//! ```rust
//! use streamweave::graph::nodes::Filter;
//!
//! // Filter items based on complex conditions
//! let filter = Filter::new(|item: &MyStruct| {
//!     item.value > 0 && item.status == "active"
//! });
//! ```
//!
//! # Design Decisions
//!
//! - **Predicate Function**: Uses a closure for flexible item filtering
//! - **True/False Logic**: Keeps items where predicate returns `true` for
//!   intuitive behavior
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`Filter`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::FilterTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that filters items based on a predicate function.
///
/// This node wraps `FilterTransformer` for use in graphs. It filters stream items,
/// only passing through items for which the predicate returns `true`.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Filter, TransformerNode};
///
/// let filter = Filter::new(|x: &i32| *x > 10);
/// let node = TransformerNode::from_transformer(
///     "filter".to_string(),
///     filter,
/// );
/// ```
pub struct Filter<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying filter transformer
  transformer: FilterTransformer<F, T>,
}

impl<F, T> Filter<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Filter` node with the specified predicate function.
  ///
  /// # Arguments
  ///
  /// * `predicate` - The predicate function used to filter items.
  pub fn new(predicate: F) -> Self {
    Self {
      transformer: FilterTransformer::new(predicate),
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

impl<F, T> Clone for Filter<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: FilterTransformer::new(self.transformer.predicate.clone()),
    }
    .with_error_strategy(self.transformer.config.error_strategy.clone())
    .with_name(
      self
        .transformer
        .config
        .name
        .clone()
        .unwrap_or_else(|| "filter".to_string()),
    )
  }
}

impl<F, T> Input for Filter<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<F, T> Output for Filter<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<F, T> Transformer for Filter<F, T>
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

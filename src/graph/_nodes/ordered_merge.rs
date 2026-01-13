//! # Ordered Merge Node
//!
//! Graph node that merges multiple input streams in a specific order. This module
//! provides [`OrderedMerge`], a graph node that combines multiple input streams
//! according to a configurable merge strategy. It supports different ordering
//! strategies: Sequential, RoundRobin, Priority, and Interleave. It wraps
//! [`OrderedMergeTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`OrderedMerge`] is useful for combining multiple input streams into a single
//! output stream in graph-based pipelines. It supports various merge strategies
//! to control how items from different streams are interleaved, making it ideal
//! for fan-in patterns and stream coordination.
//!
//! # Key Concepts
//!
//! - **Stream Merging**: Combines multiple input streams into a single output stream
//! - **Merge Strategies**: Supports different ordering strategies (Sequential, RoundRobin, Priority, Interleave)
//! - **Flexible Ordering**: Configurable merge strategy for different coordination needs
//! - **Multi-Stream Input**: Accepts items from multiple input streams simultaneously
//! - **Transformer Wrapper**: Wraps `OrderedMergeTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`OrderedMerge<T>`]**: Node that merges multiple input streams in a specific order
//! - **[`MergeStrategy`]**: Enumeration of merge strategies (Sequential, RoundRobin, Priority, Interleave)
//!
//! # Quick Start
//!
//! ## Basic Usage (Interleave)
//!
//! ```rust
//! use streamweave::graph::nodes::OrderedMerge;
//!
//! // Create an ordered merge node with default (Interleave) strategy
//! let ordered_merge = OrderedMerge::<i32>::new();
//! ```
//!
//! ## Round-Robin Merge
//!
//! ```rust
//! use streamweave::graph::nodes::OrderedMerge;
//! use streamweave::transformers::MergeStrategy;
//!
//! // Create an ordered merge node with round-robin strategy
//! let ordered_merge = OrderedMerge::<String>::new()
//!     .with_strategy(MergeStrategy::RoundRobin);
//! ```
//!
//! ## Sequential Merge
//!
//! ```rust
//! use streamweave::graph::nodes::OrderedMerge;
//! use streamweave::transformers::MergeStrategy;
//!
//! // Create an ordered merge node that processes streams sequentially
//! let ordered_merge = OrderedMerge::<i32>::new()
//!     .with_strategy(MergeStrategy::Sequential);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::OrderedMerge;
//! use streamweave::transformers::MergeStrategy;
//! use streamweave::ErrorStrategy;
//!
//! // Create an ordered merge node with error handling
//! let ordered_merge = OrderedMerge::<String>::new()
//!     .with_strategy(MergeStrategy::Interleave)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("stream-merger".to_string());
//! ```
//!
//! # Merge Strategies
//!
//! - **Interleave**: Alternates between streams fairly (default strategy)
//! - **Sequential**: Processes each stream completely before moving to the next
//! - **RoundRobin**: Takes one item from each stream in turn
//! - **Priority**: Processes streams in priority order
//!
//! # Design Decisions
//!
//! - **Strategy-Based**: Uses an enum for type-safe merge strategy selection
//! - **Default Strategy**: Uses Interleave as the default for fair alternation
//! - **Stream-Based**: Works with async streams for efficient processing
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`OrderedMerge`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`]. Multiple input streams
//! can be added using the `add_stream` method, and the merge strategy controls
//! how items from different streams are interleaved.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{MergeStrategy, OrderedMergeTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that merges multiple input streams in a specific order.
///
/// This node wraps `OrderedMergeTransformer` for use in graphs. It supports
/// different ordering strategies: Sequential, RoundRobin, Priority, and Interleave.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{OrderedMerge, TransformerNode};
/// use crate::transformers::MergeStrategy;
///
/// let ordered_merge = OrderedMerge::new()
///     .with_strategy(MergeStrategy::RoundRobin);
/// let node = TransformerNode::from_transformer(
///     "merge".to_string(),
///     ordered_merge,
/// );
/// ```
pub struct OrderedMerge<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying ordered merge transformer
  transformer: OrderedMergeTransformer<T>,
}

impl<T> OrderedMerge<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `OrderedMerge` node with default (Interleave) strategy.
  pub fn new() -> Self {
    Self {
      transformer: OrderedMergeTransformer::new(),
    }
  }

  /// Sets the merge strategy.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The merge strategy to use.
  pub fn with_strategy(mut self, strategy: MergeStrategy) -> Self {
    self.transformer = self.transformer.with_strategy(strategy);
    self
  }

  /// Adds a stream to be merged.
  ///
  /// # Arguments
  ///
  /// * `stream` - The stream to merge with the input stream.
  pub fn add_stream(&mut self, stream: Pin<Box<dyn Stream<Item = T> + Send>>) {
    self.transformer.add_stream(stream);
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

impl<T> Default for OrderedMerge<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for OrderedMerge<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for OrderedMerge<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for OrderedMerge<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for OrderedMerge<T>
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

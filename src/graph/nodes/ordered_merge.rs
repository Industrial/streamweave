//! Ordered merge node for StreamWeave graphs
//!
//! Merges multiple input streams in a specific order. Supports different
//! ordering strategies: Sequential, RoundRobin, Priority, and Interleave.

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

//! Partition node for StreamWeave graphs
//!
//! Partitions items by a key function. Splits the input stream into two output
//! streams: one for items that match the predicate and one for items that don't.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::PartitionTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that partitions items by a key function.
///
/// This node wraps `PartitionTransformer` for use in graphs. It splits the
/// input stream into two output streams: one for items that match the predicate
/// and one for items that don't.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Partition, TransformerNode};
///
/// let partition = Partition::new(|x: &i32| *x > 10);
/// let node = TransformerNode::from_transformer(
///     "partition".to_string(),
///     partition,
/// );
/// ```
pub struct Partition<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying partition transformer
  transformer: PartitionTransformer<F, T>,
}

impl<F, T> Partition<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Partition` node with the specified predicate function.
  ///
  /// # Arguments
  ///
  /// * `predicate` - The function used to determine which partition an item belongs to.
  pub fn new(predicate: F) -> Self {
    Self {
      transformer: PartitionTransformer::new(predicate),
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

impl<F, T> Clone for Partition<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: PartitionTransformer::new(self.transformer.predicate.clone()),
    }
    .with_error_strategy(self.transformer.config.error_strategy.clone())
    .with_name(
      self
        .transformer
        .config
        .name
        .clone()
        .unwrap_or_else(|| "partition".to_string()),
    )
  }
}

impl<F, T> Input for Partition<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<F, T> Output for Partition<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = (Vec<T>, Vec<T>);
  type OutputStream = Pin<Box<dyn Stream<Item = (Vec<T>, Vec<T>)> + Send>>;
}

#[async_trait]
impl<F, T> Transformer for Partition<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
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

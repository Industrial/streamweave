//! Split-at node for StreamWeave graphs
//!
//! Splits items at a specified index. Collects items and splits them into
//! two groups: items before the index and items at or after the index.

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

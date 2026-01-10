//! Sort node for StreamWeave graphs
//!
//! Sorts items in a stream. Collects all items, sorts them, and emits them
//! in sorted order.

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

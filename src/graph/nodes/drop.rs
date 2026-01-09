//! Drop node for StreamWeave graphs
//!
//! Drops items from the stream based on a predicate function. This is the inverse
//! of FilterTransformer - items where the predicate returns `true` are dropped.

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

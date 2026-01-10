//! Filter node for StreamWeave graphs
//!
//! Filters items in a stream based on a predicate function. Only items for which
//! the predicate returns `true` are passed through.

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

//! Reduce node for StreamWeave graphs
//!
//! Reduces items to a single value using an accumulator function. Applies a
//! reducer function to each item in the stream along with an accumulator.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ReduceTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that reduces items to a single value using an accumulator function.
///
/// This node wraps `ReduceTransformer` for use in graphs. It applies a reducer
/// function to each item in the stream along with an accumulator, producing
/// a final accumulated value.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Reduce, TransformerNode};
///
/// let reduce = Reduce::new(0, |acc: i32, x: i32| acc + x);
/// let node = TransformerNode::from_transformer(
///     "sum".to_string(),
///     reduce,
/// );
/// ```
pub struct Reduce<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  /// The underlying reduce transformer
  transformer: ReduceTransformer<T, Acc, F>,
}

impl<T, Acc, F> Reduce<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  /// Creates a new `Reduce` node with the specified initial accumulator and reducer function.
  ///
  /// # Arguments
  ///
  /// * `initial` - The initial value for the accumulator.
  /// * `reducer` - The function that combines the accumulator with each item.
  pub fn new(initial: Acc, reducer: F) -> Self {
    Self {
      transformer: ReduceTransformer::new(initial, reducer),
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

impl<T, Acc, F> Clone for Reduce<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: ReduceTransformer::new(
        self.transformer.accumulator.clone(),
        self.transformer.reducer.clone(),
      ),
    }
    .with_error_strategy(self.transformer.config.error_strategy.clone())
    .with_name(
      self
        .transformer
        .config
        .name
        .clone()
        .unwrap_or_else(|| "reduce".to_string()),
    )
  }
}

impl<T, Acc, F> Input for Reduce<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T, Acc, F> Output for Reduce<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  type Output = Acc;
  type OutputStream = Pin<Box<dyn Stream<Item = Acc> + Send>>;
}

#[async_trait]
impl<T, Acc, F> Transformer for Reduce<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (Acc,);

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

//! Repeat node for StreamWeave graphs
//!
//! Repeats each incoming item N times in the output stream. Useful for testing,
//! data amplification, and retry patterns.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::RepeatTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that repeats each item N times in the output stream.
///
/// This node wraps `RepeatTransformer` for use in graphs. It takes each input item
/// and emits it `count` times consecutively.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Repeat, TransformerNode};
///
/// let repeat = Repeat::new(3);
/// let node = TransformerNode::from_transformer(
///     "repeat".to_string(),
///     repeat,
/// );
/// ```
pub struct Repeat<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying repeat transformer
  transformer: RepeatTransformer<T>,
}

impl<T> Repeat<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Repeat` node that repeats each item the specified number of times.
  ///
  /// # Arguments
  ///
  /// * `count` - The number of times to repeat each item (must be > 0)
  ///
  /// # Panics
  ///
  /// Panics if `count` is 0.
  pub fn new(count: usize) -> Self {
    Self {
      transformer: RepeatTransformer::new(count),
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

impl<T> Clone for Repeat<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for Repeat<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Repeat<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Repeat<T>
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

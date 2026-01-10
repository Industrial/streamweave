//! Sample node for StreamWeave graphs
//!
//! Randomly samples items from the stream based on a probability. Each item
//! has a given probability of being passed through.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::SampleTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that samples items from the stream.
///
/// This node wraps `SampleTransformer` for use in graphs. It passes through
/// each item with a given probability, creating a random sample.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Sample, TransformerNode};
///
/// let sample = Sample::new(0.1); // 10% probability
/// let node = TransformerNode::from_transformer(
///     "sample".to_string(),
///     sample,
/// );
/// ```
pub struct Sample<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying sample transformer
  transformer: SampleTransformer<T>,
}

impl<T> Sample<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Sample` node with the specified probability.
  ///
  /// # Arguments
  ///
  /// * `probability` - The probability (0.0 to 1.0) that an item will be passed through.
  ///
  /// # Panics
  ///
  /// Panics if `probability` is not between 0.0 and 1.0 (inclusive).
  pub fn new(probability: f64) -> Self {
    Self {
      transformer: SampleTransformer::new(probability),
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

impl<T> Clone for Sample<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: SampleTransformer::new(self.transformer.probability),
    }
    .with_error_strategy(self.transformer.config.error_strategy.clone())
    .with_name(
      self
        .transformer
        .config
        .name
        .clone()
        .unwrap_or_else(|| "sample".to_string()),
    )
  }
}

impl<T> Input for Sample<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Sample<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Sample<T>
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

//! Delay node for StreamWeave graphs
//!
//! Adds a delay before emitting each item in the stream. Useful for rate limiting,
//! scheduled processing, and retry logic with exponential backoff.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::DelayTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that delays items by a specified duration.
///
/// This node wraps `DelayTransformer` for use in graphs. It adds a delay before
/// emitting each item in the stream.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Delay, TransformerNode};
/// use std::time::Duration;
///
/// let delay = Delay::new(Duration::from_secs(1));
/// let node = TransformerNode::from_transformer(
///     "delay".to_string(),
///     delay,
/// );
/// ```
pub struct Delay<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying delay transformer
  transformer: DelayTransformer<T>,
}

impl<T> Delay<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Delay` node with the specified duration.
  ///
  /// # Arguments
  ///
  /// * `duration` - The duration to delay each item before emitting it.
  ///
  /// # Returns
  ///
  /// A new `Delay` node instance.
  pub fn new(duration: std::time::Duration) -> Self {
    Self {
      transformer: DelayTransformer::new(duration),
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

impl<T> Clone for Delay<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for Delay<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Delay<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Delay<T>
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

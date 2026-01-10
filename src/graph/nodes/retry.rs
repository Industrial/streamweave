//! Retry node for StreamWeave graphs
//!
//! Retries failed operations with configurable backoff. Automatically retries
//! failed operations up to a maximum number of times.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::RetryTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::time::Duration;

/// Node that retries failed operations with configurable retry logic.
///
/// This node wraps `RetryTransformer` for use in graphs. It automatically
/// retries failed operations up to a maximum number of times with configurable
/// backoff delay.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Retry, TransformerNode};
/// use tokio::time::Duration;
///
/// let retry = Retry::new(3, Duration::from_secs(1));
/// let node = TransformerNode::from_transformer(
///     "retry".to_string(),
///     retry,
/// );
/// ```
pub struct Retry<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying retry transformer
  transformer: RetryTransformer<T>,
}

impl<T> Retry<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Retry` node with the specified maximum retries and backoff duration.
  ///
  /// # Arguments
  ///
  /// * `max_retries` - The maximum number of retry attempts.
  /// * `backoff` - The backoff duration between retry attempts.
  pub fn new(max_retries: usize, backoff: Duration) -> Self {
    Self {
      transformer: RetryTransformer::new(max_retries, backoff),
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

impl<T> Clone for Retry<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for Retry<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Retry<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Retry<T>
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

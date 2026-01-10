//! Rate limit node for StreamWeave graphs
//!
//! Rate limits items in a stream, ensuring only a specified number of items
//! are processed within a given time window.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::RateLimitTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::time::Duration;

/// Node that rate-limits items passing through.
///
/// This node wraps `RateLimitTransformer` for use in graphs. It ensures that
/// only a specified number of items are processed within a given time window.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{RateLimit, TransformerNode};
/// use tokio::time::Duration;
///
/// let rate_limit = RateLimit::new(100, Duration::from_secs(1));
/// let node = TransformerNode::from_transformer(
///     "rate_limit".to_string(),
///     rate_limit,
/// );
/// ```
pub struct RateLimit<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying rate limit transformer
  transformer: RateLimitTransformer<T>,
}

impl<T> RateLimit<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `RateLimit` node with the specified rate limit and time window.
  ///
  /// # Arguments
  ///
  /// * `rate_limit` - The maximum number of items allowed within the time window.
  /// * `time_window` - The time window for rate limiting.
  pub fn new(rate_limit: usize, time_window: Duration) -> Self {
    Self {
      transformer: RateLimitTransformer::new(rate_limit, time_window),
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

impl<T> Clone for RateLimit<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: RateLimitTransformer::new(
        self.transformer.rate_limit,
        self.transformer.time_window,
      ),
    }
    .with_error_strategy(self.transformer.config.error_strategy.clone())
    .with_name(
      self
        .transformer
        .config
        .name
        .clone()
        .unwrap_or_else(|| "rate_limit".to_string()),
    )
  }
}

impl<T> Input for RateLimit<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for RateLimit<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for RateLimit<T>
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

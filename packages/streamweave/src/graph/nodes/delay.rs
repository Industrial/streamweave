//! Transformer that delays items by a specified duration.
//!
//! Adds a delay before emitting each item in the stream. Useful for rate limiting,
//! scheduled processing, and retry logic with exponential backoff.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::marker::PhantomData;
use std::pin::Pin;

/// Transformer that delays items by a specified duration.
///
/// Adds a delay before emitting each item in the stream. Useful for rate limiting,
/// scheduled processing, and retry logic with exponential backoff.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::control_flow::Delay;
/// use streamweave::graph::node::TransformerNode;
/// use std::time::Duration;
///
/// let delay = Delay::new(Duration::from_secs(1));
/// let node = TransformerNode::from_transformer(
///     "delay".to_string(),
///     delay,
/// );
/// ```
pub struct Delay<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// Duration to delay each item
  duration: std::time::Duration,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
  /// Phantom data for type parameter
  _phantom: PhantomData<T>,
}

impl<T> Delay<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Delay` transformer with the specified duration.
  ///
  /// # Arguments
  ///
  /// * `duration` - The duration to delay each item before emitting it.
  ///
  /// # Returns
  ///
  /// A new `Delay` transformer instance.
  pub fn new(duration: std::time::Duration) -> Self {
    Self {
      duration,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
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
    let duration = self.duration;
    Box::pin(input.then(move |item| async move {
      tokio::time::sleep(duration).await;
      item
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config.error_strategy() {
      crate::error::ErrorStrategy::Stop => ErrorAction::Stop,
      crate::error::ErrorStrategy::Skip => ErrorAction::Skip,
      crate::error::ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      crate::error::ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "delay".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

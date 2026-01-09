//! Wrap transformer for StreamWeave
//!
//! Wraps items with additional metadata or envelopes. While StreamWeave automatically
//! wraps items in `Message<T>`, this transformer allows adding custom metadata or
//! wrapping items in custom envelope structures.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;

/// A transformer that wraps items with additional metadata or envelope structures.
///
/// While StreamWeave automatically wraps items in `Message<T>`, this transformer
/// allows adding custom metadata, headers, or wrapping items in custom envelope
/// structures for inter-system communication.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::WrapTransformer;
/// use std::collections::HashMap;
///
/// let mut headers = HashMap::new();
/// headers.insert("source".to_string(), "pipeline".to_string());
/// let transformer = WrapTransformer::with_headers(headers);
/// ```
pub struct WrapTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Additional headers to add to the message
  headers: Option<HashMap<String, String>>,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
}

impl<T> WrapTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `WrapTransformer` that adds no additional metadata.
  ///
  /// Items will be passed through unchanged. This is useful when you want
  /// to ensure items are properly wrapped in `Message<T>`.
  pub fn new() -> Self {
    Self {
      headers: None,
      config: TransformerConfig::default(),
    }
  }

  /// Creates a new `WrapTransformer` that adds the specified headers to messages.
  ///
  /// # Arguments
  ///
  /// * `headers` - Headers to add to each message.
  pub fn with_headers(headers: HashMap<String, String>) -> Self {
    Self {
      headers: Some(headers),
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Default for WrapTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for WrapTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      headers: self.headers.clone(),
      config: self.config.clone(),
    }
  }
}

impl<T> Input for WrapTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for WrapTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for WrapTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    // For now, just pass through items unchanged
    // In a real implementation with Message<T>, we would add headers here
    // Since we're working with raw types in transformers, we just pass through
    Box::pin(input.map(|item| item))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "wrap_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

//! String length transformer for StreamWeave
//!
//! Gets the length of strings, producing a stream of usize values.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A transformer that gets the length of strings.
///
/// Takes each input string and outputs its length as a usize.
///
/// # Example
///
/// ```rust
/// use crate::transformers::StringLengthTransformer;
///
/// let transformer = StringLengthTransformer::new();
/// // Input: ["hello", "world"]
/// // Output: [5, 5]
/// ```
pub struct StringLengthTransformer {
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringLengthTransformer {
  /// Creates a new `StringLengthTransformer`.
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Default for StringLengthTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for StringLengthTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for StringLengthTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringLengthTransformer {
  type Output = usize;
  type OutputStream = Pin<Box<dyn Stream<Item = usize> + Send>>;
}

#[async_trait]
impl Transformer for StringLengthTransformer {
  type InputPorts = (String,);
  type OutputPorts = (usize,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|s| s.len()))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
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
        .unwrap_or_else(|| "string_length_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

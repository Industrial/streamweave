//! String concat transformer for StreamWeave
//!
//! Concatenates multiple strings into a single string.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A transformer that concatenates strings.
///
/// Takes an array of strings and concatenates them into a single string.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::StringConcatTransformer;
///
/// let transformer = StringConcatTransformer::new();
/// // Input: [vec!["hello", " ", "world"]]
/// // Output: ["hello world"]
/// ```
pub struct StringConcatTransformer {
  /// Configuration for the transformer
  config: TransformerConfig<Vec<String>>,
}

impl StringConcatTransformer {
  /// Creates a new `StringConcatTransformer`.
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Vec<String>>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Default for StringConcatTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for StringConcatTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for StringConcatTransformer {
  type Input = Vec<String>;
  type InputStream = Pin<Box<dyn Stream<Item = Vec<String>> + Send>>;
}

impl Output for StringConcatTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringConcatTransformer {
  type InputPorts = (Vec<String>,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|vec| vec.concat()))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Vec<String>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Vec<String>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Vec<String>> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Vec<String>>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Vec<String>>) -> ErrorContext<Vec<String>> {
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
        .unwrap_or_else(|| "string_concat_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

//! String repeat transformer for StreamWeave
//!
//! Repeats strings a specified number of times.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A transformer that repeats strings.
///
/// Takes each input string and repeats it a specified number of times.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::StringRepeatTransformer;
///
/// let transformer = StringRepeatTransformer::new(3);
/// // Input: ["hi"]
/// // Output: ["hihihi"]
/// ```
pub struct StringRepeatTransformer {
  /// Number of times to repeat
  count: usize,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringRepeatTransformer {
  /// Creates a new `StringRepeatTransformer`.
  ///
  /// # Arguments
  ///
  /// * `count` - Number of times to repeat the string.
  pub fn new(count: usize) -> Self {
    Self {
      count,
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

impl Clone for StringRepeatTransformer {
  fn clone(&self) -> Self {
    Self {
      count: self.count,
      config: self.config.clone(),
    }
  }
}

impl Input for StringRepeatTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringRepeatTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringRepeatTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let count = self.count;
    Box::pin(input.map(move |s| s.repeat(count)))
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
        .unwrap_or_else(|| "string_repeat_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

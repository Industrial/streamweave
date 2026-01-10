//! String trim transformer for StreamWeave
//!
//! Removes whitespace from strings (leading, trailing, or both).

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// Trim mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimMode {
  /// Trim both leading and trailing whitespace
  Both,
  /// Trim only leading whitespace
  Left,
  /// Trim only trailing whitespace
  Right,
}

/// A transformer that trims whitespace from strings.
///
/// Removes whitespace from the beginning, end, or both sides of strings.
///
/// # Example
///
/// ```rust
/// use crate::transformers::{StringTrimTransformer, TrimMode};
///
/// let transformer = StringTrimTransformer::new(TrimMode::Both);
/// // Input: ["  hello world  "]
/// // Output: ["hello world"]
/// ```
pub struct StringTrimTransformer {
  /// Trim mode
  mode: TrimMode,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringTrimTransformer {
  /// Creates a new `StringTrimTransformer` with the specified mode.
  ///
  /// # Arguments
  ///
  /// * `mode` - The trim mode (Both, Left, or Right).
  pub fn new(mode: TrimMode) -> Self {
    Self {
      mode,
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

impl Clone for StringTrimTransformer {
  fn clone(&self) -> Self {
    Self {
      mode: self.mode,
      config: self.config.clone(),
    }
  }
}

impl Input for StringTrimTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringTrimTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringTrimTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let mode = self.mode;
    Box::pin(input.map(move |s| match mode {
      TrimMode::Both => s.trim().to_string(),
      TrimMode::Left => s.trim_start().to_string(),
      TrimMode::Right => s.trim_end().to_string(),
    }))
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
        .unwrap_or_else(|| "string_trim_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

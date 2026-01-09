//! String case transformer for StreamWeave
//!
//! Converts strings to lowercase or uppercase.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// Case conversion mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseMode {
  /// Convert to lowercase
  Lowercase,
  /// Convert to uppercase
  Uppercase,
}

/// A transformer that converts string case.
///
/// Converts strings to either lowercase or uppercase.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{StringCaseTransformer, CaseMode};
///
/// let transformer = StringCaseTransformer::new(CaseMode::Lowercase);
/// // Input: ["Hello World"]
/// // Output: ["hello world"]
/// ```
pub struct StringCaseTransformer {
  /// Case conversion mode
  mode: CaseMode,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringCaseTransformer {
  /// Creates a new `StringCaseTransformer` with the specified mode.
  ///
  /// # Arguments
  ///
  /// * `mode` - The case conversion mode (Lowercase or Uppercase).
  pub fn new(mode: CaseMode) -> Self {
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

impl Clone for StringCaseTransformer {
  fn clone(&self) -> Self {
    Self {
      mode: self.mode,
      config: self.config.clone(),
    }
  }
}

impl Input for StringCaseTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringCaseTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringCaseTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let mode = self.mode;
    Box::pin(input.map(move |s| match mode {
      CaseMode::Lowercase => s.to_lowercase(),
      CaseMode::Uppercase => s.to_uppercase(),
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
        .unwrap_or_else(|| "string_case_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

//! String match transformer for StreamWeave
//!
//! Filters strings that match a regex pattern.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use regex::Regex;
use std::pin::Pin;

/// A transformer that filters strings matching a regex pattern.
///
/// Only strings that match the specified regex pattern are passed through.
///
/// # Example
///
/// ```rust
/// use crate::transformers::StringMatchTransformer;
///
/// let transformer = StringMatchTransformer::new(r"^\d+$").unwrap();
/// // Input: ["123", "abc", "456"]
/// // Output: ["123", "456"]
/// ```
pub struct StringMatchTransformer {
  /// Compiled regex pattern
  pattern: Regex,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringMatchTransformer {
  /// Creates a new `StringMatchTransformer` with the specified regex pattern.
  ///
  /// # Arguments
  ///
  /// * `pattern` - The regex pattern string.
  ///
  /// # Errors
  ///
  /// Returns an error if the pattern is invalid.
  pub fn new(pattern: &str) -> Result<Self, regex::Error> {
    Ok(Self {
      pattern: Regex::new(pattern)?,
      config: TransformerConfig::default(),
    })
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

impl Clone for StringMatchTransformer {
  fn clone(&self) -> Self {
    Self {
      pattern: self.pattern.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for StringMatchTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringMatchTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringMatchTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let pattern = self.pattern.clone();
    Box::pin(input.filter(move |s| {
      let pattern = pattern.clone();
      futures::future::ready(pattern.is_match(s))
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
        .unwrap_or_else(|| "string_match_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

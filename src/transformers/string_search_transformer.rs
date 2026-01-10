//! String search transformer for StreamWeave
//!
//! Searches for regex patterns in strings, producing match information.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use regex::Regex;
use std::pin::Pin;

/// A transformer that searches for regex patterns in strings.
///
/// Searches for the first match of a regex pattern and returns the matched substring,
/// or None if no match is found.
///
/// # Example
///
/// ```rust
/// use crate::transformers::StringSearchTransformer;
///
/// let transformer = StringSearchTransformer::new(r"\d+").unwrap();
/// // Input: ["hello 123 world"]
/// // Output: [Some("123")]
/// ```
pub struct StringSearchTransformer {
  /// Compiled regex pattern
  pattern: Regex,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringSearchTransformer {
  /// Creates a new `StringSearchTransformer` with the specified regex pattern.
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

impl Clone for StringSearchTransformer {
  fn clone(&self) -> Self {
    Self {
      pattern: self.pattern.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for StringSearchTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringSearchTransformer {
  type Output = Option<String>;
  type OutputStream = Pin<Box<dyn Stream<Item = Option<String>> + Send>>;
}

#[async_trait]
impl Transformer for StringSearchTransformer {
  type InputPorts = (String,);
  type OutputPorts = (Option<String>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let pattern = self.pattern.clone();
    Box::pin(input.map(move |s| pattern.find(&s).map(|m| m.as_str().to_string())))
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
        .unwrap_or_else(|| "string_search_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

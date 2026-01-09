//! String replace transformer for StreamWeave
//!
//! Replaces substrings in strings, supporting both single and all occurrences.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A transformer that replaces substrings in strings.
///
/// Supports both replacing the first occurrence and replacing all occurrences.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::StringReplaceTransformer;
///
/// // Replace all occurrences
/// let transformer = StringReplaceTransformer::new("old", "new", true);
/// // Input: ["old old text"]
/// // Output: ["new new text"]
/// ```
pub struct StringReplaceTransformer {
  /// Pattern to find
  pattern: String,
  /// Replacement string
  replacement: String,
  /// Whether to replace all occurrences (true) or just the first (false)
  replace_all: bool,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringReplaceTransformer {
  /// Creates a new `StringReplaceTransformer`.
  ///
  /// # Arguments
  ///
  /// * `pattern` - The substring pattern to find.
  /// * `replacement` - The replacement string.
  /// * `replace_all` - If true, replace all occurrences; if false, replace only the first.
  pub fn new(
    pattern: impl Into<String>,
    replacement: impl Into<String>,
    replace_all: bool,
  ) -> Self {
    Self {
      pattern: pattern.into(),
      replacement: replacement.into(),
      replace_all,
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

impl Clone for StringReplaceTransformer {
  fn clone(&self) -> Self {
    Self {
      pattern: self.pattern.clone(),
      replacement: self.replacement.clone(),
      replace_all: self.replace_all,
      config: self.config.clone(),
    }
  }
}

impl Input for StringReplaceTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringReplaceTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringReplaceTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let pattern = self.pattern.clone();
    let replacement = self.replacement.clone();
    let replace_all = self.replace_all;
    Box::pin(input.map(move |s| {
      if replace_all {
        s.replace(&pattern, &replacement)
      } else {
        s.replacen(&pattern, &replacement, 1)
      }
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
        .unwrap_or_else(|| "string_replace_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

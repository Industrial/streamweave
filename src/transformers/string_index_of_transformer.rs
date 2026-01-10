//! String index of transformer for StreamWeave
//!
//! Finds the index of a substring in strings, producing a stream of `Option<usize>` values.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// Index search mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexOfMode {
  /// Find first occurrence
  First,
  /// Find last occurrence
  Last,
}

/// A transformer that finds the index of a substring in strings.
///
/// Takes each input string and finds the index of the specified substring,
/// producing `Option<usize>` (None if not found).
///
/// # Example
///
/// ```rust
/// use crate::transformers::{StringIndexOfTransformer, IndexOfMode};
///
/// let transformer = StringIndexOfTransformer::new("lo", IndexOfMode::First);
/// // Input: ["hello world"]
/// // Output: [Some(3)]
/// ```
pub struct StringIndexOfTransformer {
  /// Substring to find
  substring: String,
  /// Search mode
  mode: IndexOfMode,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringIndexOfTransformer {
  /// Creates a new `StringIndexOfTransformer`.
  ///
  /// # Arguments
  ///
  /// * `substring` - The substring to find.
  /// * `mode` - The search mode (First or Last).
  pub fn new(substring: impl Into<String>, mode: IndexOfMode) -> Self {
    Self {
      substring: substring.into(),
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

impl Clone for StringIndexOfTransformer {
  fn clone(&self) -> Self {
    Self {
      substring: self.substring.clone(),
      mode: self.mode,
      config: self.config.clone(),
    }
  }
}

impl Input for StringIndexOfTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringIndexOfTransformer {
  type Output = Option<usize>;
  type OutputStream = Pin<Box<dyn Stream<Item = Option<usize>> + Send>>;
}

#[async_trait]
impl Transformer for StringIndexOfTransformer {
  type InputPorts = (String,);
  type OutputPorts = (Option<usize>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let substring = self.substring.clone();
    let mode = self.mode;
    Box::pin(input.map(move |s| match mode {
      IndexOfMode::First => s.find(&substring),
      IndexOfMode::Last => s.rfind(&substring),
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
        .unwrap_or_else(|| "string_index_of_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

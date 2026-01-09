//! String split words transformer for StreamWeave
//!
//! Splits strings into words, producing a stream of words.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::stream;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A transformer that splits strings into words.
///
/// Takes each input string and splits it into words (by whitespace),
/// producing a stream of words.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::StringSplitWordsTransformer;
///
/// let transformer = StringSplitWordsTransformer::new();
/// // Input: ["hello world"]
/// // Output: ["hello", "world"]
/// ```
pub struct StringSplitWordsTransformer {
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringSplitWordsTransformer {
  /// Creates a new `StringSplitWordsTransformer`.
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

impl Default for StringSplitWordsTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for StringSplitWordsTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for StringSplitWordsTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringSplitWordsTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringSplitWordsTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.flat_map(|s| {
      let words: Vec<String> = s.split_whitespace().map(|word| word.to_string()).collect();
      stream::iter(words)
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
        .unwrap_or_else(|| "string_split_words_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

//! # String Split Lines Transformer
//!
//! Transformer that splits input strings by newline characters, producing a stream
//! where each line becomes a separate output item. Useful for processing multi-line
//! text data.
//!
//! ## Overview
//!
//! The String Split Lines Transformer provides:
//!
//! - **Line Splitting**: Splits strings by newline characters (\n)
//! - **One-to-Many**: Each input string can produce multiple output lines
//! - **Newline Handling**: Handles various newline formats
//! - **Error Handling**: Configurable error strategies
//!
//! ## Input/Output
//!
//! - **Input**: `Message<String>` - Multi-line strings to split
//! - **Output**: `Message<String>` - Individual lines from the input strings
//!
//! ## Example
//!
//! ```rust
//! use crate::transformers::StringSplitLinesTransformer;
//!
//! let transformer = StringSplitLinesTransformer::new();
//! // Input: ["line1\nline2\nline3"]
//! // Output: ["line1", "line2", "line3"]
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::stream;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A transformer that splits strings by newlines.
///
/// Takes each input string and splits it by newline characters,
/// producing a stream of lines.
///
/// # Example
///
/// ```rust
/// use crate::transformers::StringSplitLinesTransformer;
///
/// let transformer = StringSplitLinesTransformer::new();
/// // Input: ["line1\nline2\nline3"]
/// // Output: ["line1", "line2", "line3"]
/// ```
pub struct StringSplitLinesTransformer {
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringSplitLinesTransformer {
  /// Creates a new `StringSplitLinesTransformer`.
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

impl Default for StringSplitLinesTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for StringSplitLinesTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for StringSplitLinesTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringSplitLinesTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringSplitLinesTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.flat_map(|s| {
      let lines: Vec<String> = s.lines().map(|line| line.to_string()).collect();
      stream::iter(lines)
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
        .unwrap_or_else(|| "string_split_lines_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

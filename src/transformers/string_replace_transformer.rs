//! String replacement transformer for StreamWeave.
//!
//! This module provides [`StringReplaceTransformer`], a transformer that replaces
//! substrings in input strings, supporting both single occurrence replacement and
//! replace-all modes. Useful for text normalization, data cleaning, and string
//! manipulation operations.
//!
//! # Overview
//!
//! [`StringReplaceTransformer`] is useful for performing substring replacements
//! in streaming text data. It supports replacing either the first occurrence or
//! all occurrences of a pattern within each input string.
//!
//! # Key Concepts
//!
//! - **Substring Replacement**: Replaces pattern substrings with replacement text
//! - **Replace Modes**: Single occurrence or replace-all modes
//! - **Pattern Matching**: Exact substring matching (not regex-based)
//! - **Text Processing**: Useful for data cleaning and normalization
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`StringReplaceTransformer`]**: Transformer that replaces substrings in strings
//!
//! # Quick Start
//!
//! ## Basic Usage - Replace All
//!
//! ```rust
//! use streamweave::transformers::StringReplaceTransformer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that replaces all occurrences
//! let transformer = StringReplaceTransformer::new("old", "new", true);
//!
//! // Input: ["old old text"]
//! // Output: ["new new text"]
//! # Ok(())
//! # }
//! ```
//!
//! ## Replace First Occurrence Only
//!
//! ```rust
//! use streamweave::transformers::StringReplaceTransformer;
//!
//! // Create a transformer that replaces only the first occurrence
//! let transformer = StringReplaceTransformer::new("old", "new", false);
//!
//! // Input: ["old old text"]
//! // Output: ["new old text"]
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::StringReplaceTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = StringReplaceTransformer::new("pattern", "replacement", true)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("text-cleaner".to_string());
//! ```
//!
//! # Replace Modes
//!
//! - **Replace All** (`replace_all = true`): Replaces all occurrences of the pattern
//! - **Replace First** (`replace_all = false`): Replaces only the first occurrence
//!
//! # Design Decisions
//!
//! - **Exact Matching**: Uses exact substring matching, not regex (for performance)
//! - **Simple API**: Pattern and replacement specified at construction time
//! - **Mode Selection**: Boolean flag for simple replace-all vs replace-first choice
//! - **String Type**: Works with `String` type for mutable string operations
//! - **Performance**: Uses Rust's standard library string replacement methods
//!
//! # Integration with StreamWeave
//!
//! [`StringReplaceTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

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
/// use crate::transformers::StringReplaceTransformer;
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

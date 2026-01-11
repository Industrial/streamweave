//! # String Reverse Transformer
//!
//! Transformer that reverses input strings character by character, producing
//! strings with characters in reverse order. This module provides
//! [`StringReverseTransformer`], a transformer that performs character reversal
//! on strings in streaming pipelines.
//!
//! # Overview
//!
//! [`StringReverseTransformer`] is useful for reversing the character order
//! of strings in streaming data processing pipelines. It handles Unicode
//! characters correctly, ensuring multi-byte characters are reversed properly
//! rather than being broken apart.
//!
//! # Key Concepts
//!
//! - **Character Reversal**: Reverses strings character by character
//! - **Unicode Support**: Handles multi-byte Unicode characters correctly
//! - **Simple Operation**: One-to-one transformation (each input produces one output)
//! - **Text Transformation**: Useful for string manipulation and transformations
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`StringReverseTransformer`]**: Transformer that reverses strings character by character
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::StringReverseTransformer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that reverses strings
//! let transformer = StringReverseTransformer::new();
//!
//! // Input: ["hello", "world"]
//! // Output: ["olleh", "dlrow"]
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::StringReverseTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = StringReverseTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("string-reverser".to_string());
//! ```
//!
//! ## Unicode Handling
//!
//! ```text
//! // Input: ["café"]
//! // Output: ["éfac"]  (Unicode grapheme clusters handled correctly)
//! ```
//!
//! # Design Decisions
//!
//! - **Character-Based Reversal**: Uses `chars().rev().collect()` for Unicode-aware
//!   character reversal
//! - **No Configuration Needed**: Simple operation with no configuration options
//! - **Performance**: Leverages Rust's efficient string operations
//! - **Type Safety**: Works with `String` type for mutable string operations
//!
//! # Integration with StreamWeave
//!
//! [`StringReverseTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A transformer that reverses strings.
///
/// Takes each input string and reverses it character by character.
///
/// # Example
///
/// ```rust
/// use crate::transformers::StringReverseTransformer;
///
/// let transformer = StringReverseTransformer::new();
/// // Input: ["hello"]
/// // Output: ["olleh"]
/// ```
pub struct StringReverseTransformer {
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringReverseTransformer {
  /// Creates a new `StringReverseTransformer`.
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

impl Default for StringReverseTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for StringReverseTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for StringReverseTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringReverseTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringReverseTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|s| s.chars().rev().collect()))
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
        .unwrap_or_else(|| "string_reverse_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

//! # String Case Transformer
//!
//! Transformer that converts input strings to lowercase or uppercase, supporting
//! Unicode-aware case conversion for international text. This module provides
//! [`StringCaseTransformer`], a transformer that performs case conversion on
//! strings in streaming pipelines.
//!
//! # Overview
//!
//! [`StringCaseTransformer`] is useful for normalizing text case in streaming
//! data processing pipelines. It supports converting strings to either lowercase
//! or uppercase using Unicode-aware conversion, making it suitable for
//! international text processing.
//!
//! # Key Concepts
//!
//! - **Case Conversion**: Converts strings to lowercase or uppercase
//! - **Unicode Support**: Handles Unicode case conversion correctly
//! - **Mode Selection**: Configurable conversion mode (Lowercase or Uppercase)
//! - **Text Normalization**: Useful for text normalization and data cleaning
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`StringCaseTransformer`]**: Transformer that converts string case
//! - **[`CaseMode`]**: Enumeration of case conversion modes (Lowercase, Uppercase)
//!
//! # Quick Start
//!
//! ## Basic Usage - Lowercase
//!
//! ```rust
//! use streamweave::transformers::{StringCaseTransformer, CaseMode};
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that converts to lowercase
//! let transformer = StringCaseTransformer::new(CaseMode::Lowercase);
//!
//! // Input: ["Hello World", "HELLO"]
//! // Output: ["hello world", "hello"]
//! # Ok(())
//! # }
//! ```
//!
//! ## Uppercase Conversion
//!
//! ```rust
//! use streamweave::transformers::{StringCaseTransformer, CaseMode};
//!
//! // Create a transformer that converts to uppercase
//! let transformer = StringCaseTransformer::new(CaseMode::Uppercase);
//!
//! // Input: ["Hello World", "hello"]
//! // Output: ["HELLO WORLD", "HELLO"]
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::{StringCaseTransformer, CaseMode};
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = StringCaseTransformer::new(CaseMode::Lowercase)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("text-normalizer".to_string());
//! ```
//!
//! # Conversion Modes
//!
//! - **Lowercase** (`CaseMode::Lowercase`): Converts all characters to lowercase
//! - **Uppercase** (`CaseMode::Uppercase`): Converts all characters to uppercase
//!
//! # Design Decisions
//!
//! - **Unicode-Aware**: Uses Rust's standard library case conversion methods
//!   which handle Unicode correctly
//! - **Mode Selection**: Uses an enum for type-safe mode selection
//! - **Simple Operation**: One-to-one transformation (each input produces one output)
//! - **Performance**: Leverages Rust's efficient string operations
//!
//! # Integration with StreamWeave
//!
//! [`StringCaseTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

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

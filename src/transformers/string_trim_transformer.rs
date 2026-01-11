//! # String Trim Transformer
//!
//! Transformer that removes whitespace from strings, supporting leading, trailing,
//! or both-side trimming based on a configurable trim mode. This module provides
//! [`StringTrimTransformer`], a transformer that performs whitespace removal on
//! strings in streaming pipelines.
//!
//! # Overview
//!
//! [`StringTrimTransformer`] is useful for cleaning string data by removing
//! whitespace in streaming data processing pipelines. It supports trimming from
//! the leading edge, trailing edge, or both sides, making it ideal for data
//! normalization and cleaning operations.
//!
//! # Key Concepts
//!
//! - **Whitespace Removal**: Removes Unicode whitespace characters from strings
//! - **Flexible Trimming**: Supports trimming leading, trailing, or both sides
//! - **Mode Configuration**: Configurable trim mode (Both, Left, Right)
//! - **Text Normalization**: Useful for data cleaning and normalization
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`StringTrimTransformer`]**: Transformer that trims whitespace from strings
//! - **[`TrimMode`]**: Enumeration of trim modes (Both, Left, Right)
//!
//! # Quick Start
//!
//! ## Basic Usage - Trim Both Sides
//!
//! ```rust
//! use streamweave::transformers::{StringTrimTransformer, TrimMode};
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that trims both leading and trailing whitespace
//! let transformer = StringTrimTransformer::new(TrimMode::Both);
//!
//! // Input: ["  hello world  ", "  test  "]
//! // Output: ["hello world", "test"]
//! # Ok(())
//! # }
//! ```
//!
//! ## Trim Leading Only
//!
//! ```rust
//! use streamweave::transformers::{StringTrimTransformer, TrimMode};
//!
//! // Create a transformer that trims only leading whitespace
//! let transformer = StringTrimTransformer::new(TrimMode::Left);
//!
//! // Input: ["  hello world  "]
//! // Output: ["hello world  "]
//! ```
//!
//! ## Trim Trailing Only
//!
//! ```rust
//! use streamweave::transformers::{StringTrimTransformer, TrimMode};
//!
//! // Create a transformer that trims only trailing whitespace
//! let transformer = StringTrimTransformer::new(TrimMode::Right);
//!
//! // Input: ["  hello world  "]
//! // Output: ["  hello world"]
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::{StringTrimTransformer, TrimMode};
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = StringTrimTransformer::new(TrimMode::Both)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("text-trimmer".to_string());
//! ```
//!
//! # Trim Modes
//!
//! - **Both** (`TrimMode::Both`): Removes whitespace from both leading and trailing edges
//! - **Left** (`TrimMode::Left`): Removes whitespace only from the leading edge
//! - **Right** (`TrimMode::Right`): Removes whitespace only from the trailing edge
//!
//! # Design Decisions
//!
//! - **Unicode-Aware**: Uses Rust's standard library trim functions which handle
//!   Unicode whitespace correctly
//! - **Mode Selection**: Uses an enum for type-safe mode selection
//! - **Simple Operation**: One-to-one transformation (each input produces one output)
//! - **Performance**: Leverages Rust's efficient string operations
//!
//! # Integration with StreamWeave
//!
//! [`StringTrimTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// Trim mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimMode {
  /// Trim both leading and trailing whitespace
  Both,
  /// Trim only leading whitespace
  Left,
  /// Trim only trailing whitespace
  Right,
}

/// A transformer that trims whitespace from strings.
///
/// Removes whitespace from the beginning, end, or both sides of strings.
///
/// # Example
///
/// ```rust
/// use crate::transformers::{StringTrimTransformer, TrimMode};
///
/// let transformer = StringTrimTransformer::new(TrimMode::Both);
/// // Input: ["  hello world  "]
/// // Output: ["hello world"]
/// ```
pub struct StringTrimTransformer {
  /// Trim mode
  mode: TrimMode,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringTrimTransformer {
  /// Creates a new `StringTrimTransformer` with the specified mode.
  ///
  /// # Arguments
  ///
  /// * `mode` - The trim mode (Both, Left, or Right).
  pub fn new(mode: TrimMode) -> Self {
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

impl Clone for StringTrimTransformer {
  fn clone(&self) -> Self {
    Self {
      mode: self.mode,
      config: self.config.clone(),
    }
  }
}

impl Input for StringTrimTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringTrimTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringTrimTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let mode = self.mode;
    Box::pin(input.map(move |s| match mode {
      TrimMode::Both => s.trim().to_string(),
      TrimMode::Left => s.trim_start().to_string(),
      TrimMode::Right => s.trim_end().to_string(),
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
        .unwrap_or_else(|| "string_trim_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

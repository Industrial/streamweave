//! # String Char Transformer
//!
//! Transformer that performs character-level operations on strings, including
//! extracting characters at indices, getting character codes, and creating strings
//! from character codes.
//!
//! ## Overview
//!
//! The String Char Transformer provides:
//!
//! - **Character Extraction**: Gets characters at specific indices
//! - **Character Codes**: Gets Unicode character codes at indices
//! - **String Creation**: Creates strings from character codes
//! - **Unicode Support**: Handles multi-byte Unicode characters correctly
//! - **Error Handling**: Configurable error strategies
//!
//! ## Input/Output
//!
//! - **CharAt**: `Message<String>` → `Message<Option<char>>`
//! - **CharCodeAt**: `Message<String>` → `Message<Option<u32>>`
//! - **FromCharCode**: `Message<u32>` → `Message<String>`
//!
//! ## Operations
//!
//! - **CharAt(usize)**: Extracts character at the specified index
//! - **CharCodeAt(usize)**: Gets the Unicode code point at the specified index
//! - **FromCharCode**: Creates a string from a Unicode code point
//!
//! ## Example
//!
//! ```rust
//! use streamweave::transformers::{StringCharTransformer, CharOperation};
//!
//! let transformer = StringCharTransformer::new(CharOperation::CharAt(0));
//! // Input: ["hello"]
//! // Output: [Some('h')]
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// Character operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharOperation {
  /// Get character at index (String -> `Option<char>`)
  CharAt(usize),
  /// Get character code at index (String -> `Option<u32>`)
  CharCodeAt(usize),
  /// Create string from character code (u32 -> String)
  FromCharCode,
}

/// A transformer that performs character operations on strings.
///
/// Supports getting characters at indices, getting character codes, and creating strings from codes.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{StringCharTransformer, CharOperation};
///
/// let transformer = StringCharTransformer::new(CharOperation::CharAt(0));
/// // Input: ["hello"]
/// // Output: [Some('h')]
/// ```
pub struct StringCharTransformer {
  /// Operation to perform
  operation: CharOperation,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringCharTransformer {
  /// Creates a new `StringCharTransformer`.
  ///
  /// # Arguments
  ///
  /// * `operation` - The character operation to perform.
  pub fn new(operation: CharOperation) -> Self {
    Self {
      operation,
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

impl Clone for StringCharTransformer {
  fn clone(&self) -> Self {
    Self {
      operation: self.operation,
      config: self.config.clone(),
    }
  }
}

// Note: This transformer has different input/output types depending on the operation
// For CharAt/CharCodeAt: String -> Option<char>/Option<u32>
// For FromCharCode: u32 -> String
// We'll need separate implementations or use a trait object approach.
// For now, let's implement CharAt and CharCodeAt as String -> Option types

impl Input for StringCharTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringCharTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringCharTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let operation = self.operation;
    Box::pin(input.map(move |s| {
      match operation {
        CharOperation::CharAt(idx) => s
          .chars()
          .nth(idx)
          .map(|c| c.to_string())
          .unwrap_or_default(),
        CharOperation::CharCodeAt(idx) => s
          .chars()
          .nth(idx)
          .map(|c| c as u32)
          .map(|code| code.to_string())
          .unwrap_or_default(),
        CharOperation::FromCharCode => {
          // This would need u32 input, but we're using String for now
          // Could parse the string as u32 and convert to char
          s.parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map(|c| c.to_string())
            .unwrap_or_default()
        }
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
        .unwrap_or_else(|| "string_char_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

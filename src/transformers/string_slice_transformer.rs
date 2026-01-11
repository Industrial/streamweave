//! # String Slice Transformer
//!
//! Transformer that extracts substrings from input strings using start and optional
//! end indices, similar to Rust's string slicing operations.
//!
//! ## Overview
//!
//! The String Slice Transformer provides:
//!
//! - **Substring Extraction**: Extracts substrings using start and end indices
//! - **Flexible End Index**: Optional end index (None extracts to end of string)
//! - **Index Validation**: Handles out-of-bounds indices gracefully
//! - **Error Handling**: Configurable error strategies
//!
//! ## Input/Output
//!
//! - **Input**: `Message<String>` - Strings to slice
//! - **Output**: `Message<String>` - Extracted substrings
//!
//! ## Example
//!
//! ```rust
//! use crate::transformers::StringSliceTransformer;
//!
//! let transformer = StringSliceTransformer::new(0, Some(5));
//! // Input: ["hello world"]
//! // Output: ["hello"]
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A transformer that extracts substrings from strings.
///
/// Extracts a substring using start and optional end indices.
/// If end is None, extracts from start to the end of the string.
///
/// # Example
///
/// ```rust
/// use crate::transformers::StringSliceTransformer;
///
/// let transformer = StringSliceTransformer::new(0, Some(5));
/// // Input: ["hello world"]
/// // Output: ["hello"]
/// ```
pub struct StringSliceTransformer {
  /// Start index (inclusive)
  start: usize,
  /// End index (exclusive), None means to end of string
  end: Option<usize>,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringSliceTransformer {
  /// Creates a new `StringSliceTransformer`.
  ///
  /// # Arguments
  ///
  /// * `start` - Start index (inclusive).
  /// * `end` - End index (exclusive), or None to extract to end of string.
  pub fn new(start: usize, end: Option<usize>) -> Self {
    Self {
      start,
      end,
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

impl Clone for StringSliceTransformer {
  fn clone(&self) -> Self {
    Self {
      start: self.start,
      end: self.end,
      config: self.config.clone(),
    }
  }
}

impl Input for StringSliceTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringSliceTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringSliceTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let start = self.start;
    let end = self.end;
    Box::pin(input.map(move |s| {
      let len = s.len();
      let start_idx = start.min(len);
      let end_idx = end.map(|e| e.min(len)).unwrap_or(len);
      if start_idx >= end_idx {
        String::new()
      } else {
        s.chars()
          .skip(start_idx)
          .take(end_idx - start_idx)
          .collect()
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
        .unwrap_or_else(|| "string_slice_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

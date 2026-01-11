//! Array length transformer for getting array sizes.
//!
//! This module provides [`ArrayLengthTransformer`], a transformer that extracts the
//! length (number of elements) from arrays. It converts each input array to its length
//! as a number, useful for validation, filtering, and conditional processing.
//!
//! # Overview
//!
//! [`ArrayLengthTransformer`] takes each input array and outputs its length as a JSON
//! number. This is useful for filtering arrays by size, validating array lengths, or
//! computing statistics about array sizes in a stream.
//!
//! # Key Concepts
//!
//! - **Length Extraction**: Returns the number of elements in arrays
//! - **Number Output**: Outputs length as a JSON number value
//! - **JSON Processing**: Works with JSON Value arrays
//! - **Simple Operation**: Direct length calculation
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`ArrayLengthTransformer`]**: Transformer that extracts array lengths
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::ArrayLengthTransformer;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer
//! let transformer = ArrayLengthTransformer::new();
//!
//! // Input: [[1, 2, 3]]
//! // Output: [3]  (length of the array)
//!
//! // Input: [[]]
//! // Output: [0]  (empty array has length 0)
//! # Ok(())
//! # }
//! ```
//!
//! # Design Decisions
//!
//! - **Direct Length**: Uses array length directly (no validation needed)
//! - **Number Type**: Returns length as JSON number for numeric operations
//! - **Zero for Empty**: Empty arrays return length 0
//! - **Simple Operation**: Minimal processing overhead
//!
//! # Integration with StreamWeave
//!
//! [`ArrayLengthTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// A transformer that gets the length of arrays.
///
/// Takes each input array and outputs its length as a number.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::ArrayLengthTransformer;
///
/// let transformer = ArrayLengthTransformer::new();
/// // Input: [[1, 2, 3]]
/// // Output: [3]
/// ```
pub struct ArrayLengthTransformer {
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ArrayLengthTransformer {
  /// Creates a new `ArrayLengthTransformer`.
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Value>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Default for ArrayLengthTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ArrayLengthTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for ArrayLengthTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ArrayLengthTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ArrayLengthTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|value| {
      if let Value::Array(arr) = value {
        Value::Number(arr.len().into())
      } else {
        Value::Number(0.into())
      }
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Value>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Value> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Value> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Value>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Value>) -> ErrorContext<Value> {
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
        .unwrap_or_else(|| "array_length_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

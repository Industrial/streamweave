//! Array concatenation transformer for combining arrays.
//!
//! This module provides [`ArrayConcatTransformer`], a transformer that concatenates
//! multiple arrays into a single array. It takes an array of arrays as input and
//! produces a single concatenated array as output.
//!
//! # Overview
//!
//! [`ArrayConcatTransformer`] is useful for combining arrays from multiple sources
//! or flattening nested array structures. It processes JSON arrays and concatenates
//! their elements into a single array.
//!
//! # Key Concepts
//!
//! - **Array Concatenation**: Combines multiple arrays into one
//! - **JSON Processing**: Works with `serde_json::Value` arrays
//! - **Nested Arrays**: Handles arrays of arrays
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`ArrayConcatTransformer`]**: Transformer that concatenates arrays
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::ArrayConcatTransformer;
//! use streamweave::PipelineBuilder;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer
//! let transformer = ArrayConcatTransformer::new();
//!
//! // Input: json!([[1, 2], [3, 4]])
//! // Output: json!([1, 2, 3, 4])
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::ArrayConcatTransformer;
//! use streamweave::ErrorStrategy;
//! use serde_json::Value;
//!
//! // Create a transformer with error handling strategy
//! let transformer = ArrayConcatTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("concat-transformer".to_string());
//! ```
//!
//! # Behavior
//!
//! The transformer takes an array of arrays as input and concatenates all sub-arrays
//! into a single array. Non-array values are passed through unchanged.
//!
//! # Design Decisions
//!
//! - **JSON Value Type**: Uses `serde_json::Value` for flexible JSON array handling
//! - **Simple Concatenation**: Directly concatenates array elements
//! - **Type Safety**: Validates input is an array before processing
//! - **Pass-Through**: Non-array values are passed through unchanged
//!
//! # Integration with StreamWeave
//!
//! [`ArrayConcatTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// A transformer that concatenates arrays.
///
/// Takes an array of arrays and concatenates them into a single array.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::ArrayConcatTransformer;
///
/// let transformer = ArrayConcatTransformer::new();
/// // Input: [[[1, 2], [3, 4]]]
/// // Output: [[1, 2, 3, 4]]
/// ```
pub struct ArrayConcatTransformer {
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ArrayConcatTransformer {
  /// Creates a new `ArrayConcatTransformer`.
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

impl Default for ArrayConcatTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ArrayConcatTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for ArrayConcatTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ArrayConcatTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ArrayConcatTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|value| {
      if let Value::Array(arr) = value {
        let mut result = Vec::new();
        for item in arr {
          if let Value::Array(sub_arr) = item {
            result.extend(sub_arr);
          }
        }
        Value::Array(result)
      } else {
        value
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
        .unwrap_or_else(|| "array_concat_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

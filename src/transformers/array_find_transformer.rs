//! Array find transformer for locating elements or indices in arrays.
//!
//! This module provides [`ArrayFindTransformer`], a transformer that finds elements
//! or indices in arrays based on value equality. It supports two operations: finding
//! the element itself or finding its index position.
//!
//! # Overview
//!
//! [`ArrayFindTransformer`] searches arrays for a specific value and either returns
//! the matching element (Find operation) or its index (FindIndex operation). This is
//! useful for conditional processing, validation, and array manipulation workflows.
//!
//! # Key Concepts
//!
//! - **Find Operation**: Returns the first matching element or `Null` if not found
//! - **FindIndex Operation**: Returns the index of the first match as a number or `Null`
//! - **Value Matching**: Uses exact equality to find matching values
//! - **JSON Processing**: Works with JSON Value arrays
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`ArrayFindTransformer`]**: Transformer that finds elements or indices
//! - **[`FindOperation`]**: Enum specifying whether to find element or index
//!
//! # Quick Start
//!
//! ## Finding Elements
//!
//! ```rust
//! use streamweave::transformers::{ArrayFindTransformer, FindOperation};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that finds the element
//! let transformer = ArrayFindTransformer::new(FindOperation::Find, json!(15));
//!
//! // Input: [[1, 5, 15, 20]]
//! // Output: [15]  (the matching element)
//! # Ok(())
//! # }
//! ```
//!
//! ## Finding Indices
//!
//! ```rust
//! use streamweave::transformers::{ArrayFindTransformer, FindOperation};
//! use serde_json::json;
//!
//! // Create a transformer that finds the index
//! let transformer = ArrayFindTransformer::new(FindOperation::FindIndex, json!(15));
//!
//! // Input: [[1, 5, 15, 20]]
//! // Output: [2]  (index of the matching element)
//! ```
//!
//! # Design Decisions
//!
//! - **Dual Operation**: Supports both element and index finding for flexibility
//! - **First Match**: Returns the first matching element/index (like JavaScript Array.find)
//! - **Null for Not Found**: Returns `Value::Null` when no match is found
//! - **Exact Matching**: Uses exact value equality for matching
//!
//! # Integration with StreamWeave
//!
//! [`ArrayFindTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Find operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindOperation {
  /// Find element (returns Value or Null)
  Find,
  /// Find index (returns number or Null)
  FindIndex,
}

/// A transformer that finds elements or indices in arrays.
///
/// Supports finding elements or their indices by value equality.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{ArrayFindTransformer, FindOperation};
///
/// // Find element
/// let transformer = ArrayFindTransformer::new(FindOperation::Find, serde_json::json!(15));
/// // Input: [[1, 5, 15, 20]]
/// // Output: [15]
/// ```
pub struct ArrayFindTransformer {
  /// Operation to perform
  operation: FindOperation,
  /// Value to find
  search_value: Value,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ArrayFindTransformer {
  /// Creates a new `ArrayFindTransformer`.
  ///
  /// # Arguments
  ///
  /// * `operation` - The operation to perform (Find or FindIndex).
  /// * `search_value` - The value to find.
  pub fn new(operation: FindOperation, search_value: Value) -> Self {
    Self {
      operation,
      search_value,
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

impl Clone for ArrayFindTransformer {
  fn clone(&self) -> Self {
    Self {
      operation: self.operation,
      search_value: self.search_value.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for ArrayFindTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ArrayFindTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ArrayFindTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let operation = self.operation;
    let search_value = self.search_value.clone();
    Box::pin(input.map(move |value| {
      if let Value::Array(arr) = value {
        match operation {
          FindOperation::Find => arr
            .iter()
            .find(|v| *v == &search_value)
            .cloned()
            .unwrap_or(Value::Null),
          FindOperation::FindIndex => arr
            .iter()
            .position(|v| v == &search_value)
            .map(|idx| Value::Number(idx.into()))
            .unwrap_or(Value::Null),
        }
      } else {
        Value::Null
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
        .unwrap_or_else(|| "array_find_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

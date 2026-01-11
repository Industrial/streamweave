//! Array modify transformer for array manipulation operations.
//!
//! This module provides [`ArrayModifyTransformer`], a transformer that modifies arrays
//! using common array operations: Push, Pop, Shift, and Unshift. These operations allow
//! you to add or remove elements from the beginning or end of arrays.
//!
//! # Overview
//!
//! [`ArrayModifyTransformer`] performs array modification operations similar to JavaScript's
//! Array methods. It supports adding elements (Push, Unshift) and removing elements (Pop, Shift),
//! making it useful for array manipulation in streaming pipelines.
//!
//! # Key Concepts
//!
//! - **Push**: Adds an element to the end of the array
//! - **Pop**: Removes and returns the last element
//! - **Shift**: Removes and returns the first element
//! - **Unshift**: Adds an element to the beginning of the array
//! - **Value Handling**: Push/Unshift require a value, Pop/Shift return the removed element
//! - **JSON Processing**: Works with JSON Value arrays
//!
//! # Core Types
//!
//! - **[`ArrayModifyTransformer`]**: Transformer that modifies arrays
//! - **[`ArrayModifyOperation`]**: Enum specifying the operation type
//!
//! # Quick Start
//!
//! ## Push Operation
//!
//! ```rust
//! use streamweave::transformers::{ArrayModifyTransformer, ArrayModifyOperation};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that pushes an element
//! let transformer = ArrayModifyTransformer::new(
//!     ArrayModifyOperation::Push,
//!     Some(json!(4))
//! );
//!
//! // Input: [[1, 2, 3]]
//! // Output: [[1, 2, 3, 4]]
//! # Ok(())
//! # }
//! ```
//!
//! ## Pop Operation
//!
//! ```rust
//! use streamweave::transformers::{ArrayModifyTransformer, ArrayModifyOperation};
//!
//! // Create a transformer that pops the last element
//! let transformer = ArrayModifyTransformer::new(
//!     ArrayModifyOperation::Pop,
//!     None
//! );
//! // Input: [[1, 2, 3]]
//! // Output: [3]  (the popped element)
//! ```
//!
//! # Design Decisions
//!
//! - **JavaScript-like Operations**: Mirrors JavaScript Array methods for familiarity
//! - **Optional Value**: Value parameter is optional (only needed for Push/Unshift)
//! - **Returns Modified Array or Element**: Push/Unshift return modified array, Pop/Shift return element
//! - **In-Place Style**: Operations modify arrays conceptually (though creates new arrays)
//!
//! # Integration with StreamWeave
//!
//! [`ArrayModifyTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Array modification operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayModifyOperation {
  /// Push element to end (requires value)
  Push,
  /// Pop element from end (returns removed element)
  Pop,
  /// Shift element from start (returns removed element)
  Shift,
  /// Unshift element to start (requires value)
  Unshift,
}

/// A transformer that modifies arrays.
///
/// Supports Push, Pop, Shift, and Unshift operations.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{ArrayModifyTransformer, ArrayModifyOperation};
///
/// // Push element
/// let transformer = ArrayModifyTransformer::new(ArrayModifyOperation::Push, Some(Value::Number(4.into())));
/// // Input: [[1, 2, 3]]
/// // Output: [[1, 2, 3, 4]]
/// ```
pub struct ArrayModifyTransformer {
  /// Operation to perform
  operation: ArrayModifyOperation,
  /// Value to add (for Push/Unshift)
  value: Option<Value>,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ArrayModifyTransformer {
  /// Creates a new `ArrayModifyTransformer`.
  ///
  /// # Arguments
  ///
  /// * `operation` - The operation to perform (Push, Pop, Shift, or Unshift).
  /// * `value` - Optional value to add (required for Push/Unshift, ignored for Pop/Shift).
  pub fn new(operation: ArrayModifyOperation, value: Option<Value>) -> Self {
    Self {
      operation,
      value,
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

impl Clone for ArrayModifyTransformer {
  fn clone(&self) -> Self {
    Self {
      operation: self.operation,
      value: self.value.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for ArrayModifyTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ArrayModifyTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ArrayModifyTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let operation = self.operation;
    let value = self.value.clone();
    Box::pin(input.map(move |mut val| {
      if let Value::Array(ref mut arr) = val {
        match operation {
          ArrayModifyOperation::Push => {
            if let Some(v) = value.clone() {
              arr.push(v);
            }
            val
          }
          ArrayModifyOperation::Pop => arr.pop().unwrap_or(Value::Null),
          ArrayModifyOperation::Shift => {
            if arr.is_empty() {
              Value::Null
            } else {
              arr.remove(0)
            }
          }
          ArrayModifyOperation::Unshift => {
            if let Some(v) = value.clone() {
              arr.insert(0, v);
            }
            val
          }
        }
      } else {
        val
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
        .unwrap_or_else(|| "array_modify_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

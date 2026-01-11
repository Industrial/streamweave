//! Array reverse transformer for reversing array element order.
//!
//! This module provides [`ArrayReverseTransformer`], a transformer that reverses
//! the order of elements in arrays. It takes each input array and produces a new
//! array with elements in reverse order.
//!
//! # Overview
//!
//! [`ArrayReverseTransformer`] reverses array element order, similar to JavaScript's
//! `Array.reverse()`. The first element becomes the last, and the last becomes the first.
//! This is useful for processing arrays in reverse order or reversing data sequences.
//!
//! # Key Concepts
//!
//! - **Element Reversal**: Reverses the order of all elements in arrays
//! - **In-Place Style**: Conceptually modifies arrays (creates new reversed arrays)
//! - **JSON Processing**: Works with JSON Value arrays
//! - **Simple Operation**: Direct array reversal
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`ArrayReverseTransformer`]**: Transformer that reverses arrays
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::ArrayReverseTransformer;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer
//! let transformer = ArrayReverseTransformer::new();
//!
//! // Input: [[1, 2, 3]]
//! // Output: [[3, 2, 1]]
//!
//! // Input: [["a", "b", "c"]]
//! // Output: [["c", "b", "a"]]
//! # Ok(())
//! # }
//! ```
//!
//! # Design Decisions
//!
//! - **Complete Reversal**: Reverses all elements, not just a subset
//! - **New Array Creation**: Creates new arrays (doesn't mutate input)
//! - **Simple Operation**: Direct element reordering
//! - **Similar to Array.reverse**: Follows JavaScript Array.reverse() semantics
//!
//! # Integration with StreamWeave
//!
//! [`ArrayReverseTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// A transformer that reverses arrays.
///
/// Takes each input array and reverses its order.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::ArrayReverseTransformer;
///
/// let transformer = ArrayReverseTransformer::new();
/// // Input: [[1, 2, 3]]
/// // Output: [[3, 2, 1]]
/// ```
pub struct ArrayReverseTransformer {
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ArrayReverseTransformer {
  /// Creates a new `ArrayReverseTransformer`.
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

impl Default for ArrayReverseTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ArrayReverseTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for ArrayReverseTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ArrayReverseTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ArrayReverseTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|value| {
      if let Value::Array(mut arr) = value {
        arr.reverse();
        Value::Array(arr)
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
        .unwrap_or_else(|| "array_reverse_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

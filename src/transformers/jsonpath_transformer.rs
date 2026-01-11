//! JSONPath transformer for querying JSON objects using JSONPath expressions.
//!
//! This module provides [`JsonPathTransformer`] and [`JsonPathOperation`], types for
//! querying JSON objects using JSONPath expressions in StreamWeave pipelines. It
//! supports both Get (extract values) and Compare (compare values) operations,
//! making it ideal for JSON data extraction and filtering. It implements the
//! [`Transformer`] trait for use in StreamWeave pipelines and graphs.
//!
//! # Overview
//!
//! [`JsonPathTransformer`] is useful for querying and extracting data from JSON
//! objects in StreamWeave pipelines. It processes JSON objects and applies JSONPath
//! expressions to extract or compare values, making it ideal for flexible JSON data
//! processing.
//!
//! # Key Concepts
//!
//! - **JSONPath Expressions**: Uses JSONPath expressions to query JSON objects
//! - **Get Operation**: Extracts values from JSON objects based on JSONPath
//! - **Compare Operation**: Compares values from JSON objects based on JSONPath
//! - **Basic Implementation**: Provides basic JSONPath support for common use cases
//!
//! # Core Types
//!
//! - **[`JsonPathTransformer`]**: Transformer that queries JSON objects using JSONPath
//! - **[`JsonPathOperation`]**: Enum representing different JSONPath operations
//!
//! # Quick Start
//!
//! ## Basic Usage (Get Operation)
//!
//! ```rust
//! use streamweave::transformers::{JsonPathTransformer, JsonPathOperation};
//!
//! // Extract a value using JSONPath
//! let transformer = JsonPathTransformer::new(
//!     "$.name",
//!     JsonPathOperation::Get,
//!     None
//! );
//! ```
//!
//! ## Compare Operation
//!
//! ```rust
//! use streamweave::transformers::{JsonPathTransformer, JsonPathOperation};
//! use serde_json::json;
//!
//! // Compare a value using JSONPath
//! let transformer = JsonPathTransformer::new(
//!     "$.status",
//!     JsonPathOperation::Compare,
//!     Some(json!("active"))
//! );
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::{JsonPathTransformer, JsonPathOperation};
//! use streamweave::ErrorStrategy;
//!
//! // Create a JSONPath transformer with error handling
//! let transformer = JsonPathTransformer::new(
//!     "$.name",
//!     JsonPathOperation::Get,
//!     None
//! )
//! .with_error_strategy(ErrorStrategy::Skip)
//! .with_name("jsonpath-query".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSONPath Support**: Provides basic JSONPath expression evaluation
//! - **Multiple Operations**: Supports both Get and Compare operations for
//!   flexibility
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible
//!   JSON handling
//! - **Transformer Trait**: Implements `Transformer` for integration with
//!   pipeline system
//!
//! # Integration with StreamWeave
//!
//! [`JsonPathTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// JSONPath operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonPathOperation {
  /// Get value(s) by JSONPath
  Get,
  /// Compare value by JSONPath
  Compare,
}

/// A transformer that queries JSON objects using JSONPath expressions.
///
/// Supports getting values and comparing values using JSONPath.
/// Note: This is a basic implementation. For full JSONPath support,
/// consider using a dedicated JSONPath library.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{JsonPathTransformer, JsonPathOperation};
///
/// let transformer = JsonPathTransformer::new("$.name", JsonPathOperation::Get);
/// // Input: [{"name": "John", "age": 30}]
/// // Output: ["John"]
/// ```
pub struct JsonPathTransformer {
  /// JSONPath expression
  path: String,
  /// Operation to perform
  operation: JsonPathOperation,
  /// Comparison value (for Compare operation)
  compare_value: Option<Value>,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl JsonPathTransformer {
  /// Creates a new `JsonPathTransformer`.
  ///
  /// # Arguments
  ///
  /// * `path` - The JSONPath expression (e.g., "$.name", "$.users\[0\].name").
  /// * `operation` - The operation to perform (Get or Compare).
  /// * `compare_value` - Optional value to compare against (for Compare operation).
  pub fn new(
    path: impl Into<String>,
    operation: JsonPathOperation,
    compare_value: Option<Value>,
  ) -> Self {
    Self {
      path: path.into(),
      operation,
      compare_value,
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

  /// Simple JSONPath resolver for basic paths like $.key or $.key.subkey
  /// This is a simplified implementation. For full JSONPath support, use a library.
  fn resolve_path(&self, obj: &Value) -> Option<Value> {
    let path = self.path.trim();
    if !path.starts_with('$') {
      return None;
    }

    let parts: Vec<&str> = path[1..].split('.').filter(|s| !s.is_empty()).collect();

    let mut current = obj;
    for part in parts {
      let part = part.trim();
      if part.is_empty() {
        continue;
      }

      // Handle array indexing like [0] or [1]
      if let Some(bracket_start) = part.find('[') {
        let key = &part[..bracket_start];
        if !key.is_empty() {
          current = current.get(key)?;
        }
        let bracket_end = part.find(']')?;
        let index_str = &part[bracket_start + 1..bracket_end];
        let index: usize = index_str.parse().ok()?;
        current = current.as_array()?.get(index)?;
      } else {
        current = current.get(part)?;
      }
    }

    Some(current.clone())
  }
}

impl Clone for JsonPathTransformer {
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      operation: self.operation,
      compare_value: self.compare_value.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for JsonPathTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for JsonPathTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for JsonPathTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let path = self.path.clone();
    let operation = self.operation;
    let compare_value = self.compare_value.clone();
    let compare_value_clone = compare_value.clone();
    Box::pin(input.filter_map(move |obj| {
      let path = path.clone();
      let compare_value = compare_value_clone.clone();
      let transformer = JsonPathTransformer {
        path,
        operation,
        compare_value: compare_value.clone(),
        config: TransformerConfig::default(),
      };
      async move {
        match operation {
          JsonPathOperation::Get => transformer.resolve_path(&obj),
          JsonPathOperation::Compare => {
            let resolved = transformer.resolve_path(&obj);
            let matches = resolved
              .and_then(|r| compare_value.as_ref().map(|cv| r == *cv))
              .unwrap_or(false);
            if matches { Some(obj) } else { None }
          }
        }
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
        .unwrap_or_else(|| "jsonpath_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

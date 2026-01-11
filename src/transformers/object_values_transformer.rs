//! # Object Values Transformer
//!
//! Transformer that extracts values from JSON objects, producing a stream of JSON arrays
//! containing the values from each input object. Useful for extracting object values
//! for further processing or analysis.
//!
//! This module provides [`ObjectValuesTransformer`], a transformer that takes JSON objects
//! as input and outputs JSON arrays containing only the values (not the keys) from each
//! input object. The order of values in the output array matches the order they appear
//! in the input object.
//!
//! # Overview
//!
//! [`ObjectValuesTransformer`] is useful for extracting values from JSON objects when
//! you only need the values without the keys. It transforms each input object into an
//! array containing its values, making it easier to process values independently of
//! their keys.
//!
//! # Key Concepts
//!
//! - **Value Extraction**: Extracts values from JSON objects, discarding keys
//! - **Array Output**: Outputs JSON arrays containing object values
//! - **Order Preservation**: Preserves the order of values as they appear in the input object
//! - **Non-Object Handling**: Returns empty arrays for non-object inputs
//! - **Error Handling**: Configurable error strategies for processing failures
//!
//! # Core Types
//!
//! - **[`ObjectValuesTransformer`]**: Transformer that extracts values from JSON objects
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::ObjectValuesTransformer;
//! use serde_json::json;
//!
//! let transformer = ObjectValuesTransformer::new();
//! // Input: [{"name": "John", "age": 30}]
//! // Output: [[json!("John"), json!(30)]]
//! ```
//!
//! ## With Multiple Objects
//!
//! ```rust
//! use streamweave::transformers::ObjectValuesTransformer;
//! use serde_json::json;
//!
//! let transformer = ObjectValuesTransformer::new();
//! // Input: [
//! //     {"name": "John", "age": 30},
//! //     {"name": "Jane", "age": 25}
//! // ]
//! // Output: [
//! //     [json!("John"), json!(30)],
//! //     [json!("Jane"), json!(25)]
//! // ]
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::ObjectValuesTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = ObjectValuesTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("value-extractor".to_string());
//! ```
//!
//! ## Handling Non-Objects
//!
//! ```rust
//! use streamweave::transformers::ObjectValuesTransformer;
//! use serde_json::json;
//!
//! let transformer = ObjectValuesTransformer::new();
//! // Input: [json!("string"), json!(42), json!({"key": "value"})]
//! // Output: [json!([]), json!([]), [json!("value")]]
//! // Non-object values result in empty arrays
//! ```
//!
//! # Design Decisions
//!
//! - **Value-Only Output**: Extracts only values, discarding keys for simplicity
//! - **Array Representation**: Outputs arrays to represent collections of values
//! - **Non-Object Handling**: Returns empty arrays for non-object inputs (graceful degradation)
//! - **Order Preservation**: Maintains the order of values as they appear in objects
//! - **JSON Value Type**: Works with `serde_json::Value` for flexible JSON processing
//!
//! # Integration with StreamWeave
//!
//! [`ObjectValuesTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// A transformer that extracts values from JSON objects.
///
/// Takes each input object and outputs an array of its values.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::ObjectValuesTransformer;
///
/// let transformer = ObjectValuesTransformer::new();
/// // Input: [{"name": "John", "age": 30}]
/// // Output: [["John", 30]]
/// ```
pub struct ObjectValuesTransformer {
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ObjectValuesTransformer {
  /// Creates a new `ObjectValuesTransformer`.
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

impl Default for ObjectValuesTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ObjectValuesTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for ObjectValuesTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ObjectValuesTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ObjectValuesTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|obj| {
      if let Some(map) = obj.as_object() {
        let values: Vec<Value> = map.values().cloned().collect();
        Value::Array(values)
      } else {
        Value::Array(vec![])
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
        .unwrap_or_else(|| "object_values_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

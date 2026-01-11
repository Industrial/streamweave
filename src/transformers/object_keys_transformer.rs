//! Object keys transformer for StreamWeave.
//!
//! This module provides [`ObjectKeysTransformer`], a transformer that extracts
//! keys from JSON objects, producing arrays of key strings. It processes JSON
//! objects and extracts all property keys, making it ideal for object inspection,
//! key enumeration, and key-based operations.
//!
//! # Overview
//!
//! [`ObjectKeysTransformer`] is useful for extracting keys from JSON objects in
//! streaming pipelines. It processes JSON objects and produces arrays of their
//! property keys, similar to JavaScript's `Object.keys()` function.
//!
//! # Key Concepts
//!
//! - **Key Extraction**: Extracts all keys from JSON objects
//! - **Array Output**: Produces arrays of key strings
//! - **Object Inspection**: Enables inspection of object structure
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`ObjectKeysTransformer`]**: Transformer that extracts object keys
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::ObjectKeysTransformer;
//! use streamweave::PipelineBuilder;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that extracts object keys
//! let transformer = ObjectKeysTransformer::new();
//!
//! // Input: [json!({"name": "John", "age": 30})]
//! // Output: [json!(["name", "age"])]
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::ObjectKeysTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = ObjectKeysTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("keys-extractor".to_string());
//! ```
//!
//! # Behavior
//!
//! The transformer extracts all property keys from each input JSON object and
//! produces an array of key strings. Non-object values produce empty arrays.
//!
//! # Design Decisions
//!
//! - **Object-Only**: Works with JSON objects, returns empty array for non-objects
//! - **String Keys**: Produces string keys for compatibility with JSON semantics
//! - **Array Output**: Produces arrays of keys for easy iteration
//! - **Simple Extraction**: Focuses solely on key extraction for clarity
//!
//! # Integration with StreamWeave
//!
//! [`ObjectKeysTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// A transformer that extracts keys from JSON objects.
///
/// Takes each input object and outputs an array of its keys as strings.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::ObjectKeysTransformer;
///
/// let transformer = ObjectKeysTransformer::new();
/// // Input: [{"name": "John", "age": 30}]
/// // Output: [["name", "age"]]
/// ```
pub struct ObjectKeysTransformer {
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ObjectKeysTransformer {
  /// Creates a new `ObjectKeysTransformer`.
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

impl Default for ObjectKeysTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ObjectKeysTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for ObjectKeysTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ObjectKeysTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ObjectKeysTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|obj| {
      if let Some(map) = obj.as_object() {
        let keys: Vec<Value> = map.keys().map(|k| Value::String(k.clone())).collect();
        Value::Array(keys)
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
        .unwrap_or_else(|| "object_keys_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

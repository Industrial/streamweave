//! Object merge transformer for StreamWeave.
//!
//! This module provides [`ObjectMergeTransformer`], a transformer that merges
//! multiple JSON objects from an array into a single object. Later objects override
//! earlier ones for duplicate keys, enabling object composition and property
//! overriding operations.
//!
//! # Overview
//!
//! [`ObjectMergeTransformer`] is useful for merging JSON objects in streaming
//! pipelines. It processes arrays of objects and merges them into a single object,
//! with later objects taking precedence for duplicate keys.
//!
//! # Key Concepts
//!
//! - **Object Merging**: Merges arrays of objects into single objects
//! - **Property Override**: Later objects override earlier ones for duplicate keys
//! - **Shallow Merging**: Performs shallow merge (does not deep merge nested objects)
//! - **Array Input**: Expects arrays of objects as input
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`ObjectMergeTransformer`]**: Transformer that merges objects
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::ObjectMergeTransformer;
//! use streamweave::PipelineBuilder;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that merges objects
//! let transformer = ObjectMergeTransformer::new();
//!
//! // Input: [json!([{"a": 1}, {"b": 2}, {"a": 3}])]
//! // Output: [json!({"a": 3, "b": 2})]
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::ObjectMergeTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = ObjectMergeTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("object-merger".to_string());
//! ```
//!
//! # Merge Behavior
//!
//! Objects are merged left-to-right, with later objects taking precedence for
//! duplicate keys. This enables property overriding and object composition.
//! Non-object values in the array are skipped during merging.
//!
//! # Design Decisions
//!
//! - **Shallow Merge**: Performs shallow merge (does not recursively merge nested objects)
//! - **Property Override**: Later objects override earlier ones for predictable behavior
//! - **Array Input**: Expects arrays of objects for merging
//! - **Non-Object Skip**: Skips non-object values in the array
//! - **Simple Merging**: Focuses on straightforward object merging
//!
//! # Integration with StreamWeave
//!
//! [`ObjectMergeTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::{Map, Value};
use std::pin::Pin;

/// A transformer that merges JSON objects.
///
/// Takes an array of objects and merges them into a single object.
/// Later objects override earlier ones for duplicate keys.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::ObjectMergeTransformer;
///
/// let transformer = ObjectMergeTransformer::new();
/// // Input: [[{"a": 1}, {"b": 2}, {"a": 3}]]
/// // Output: [{"a": 3, "b": 2}]
/// ```
pub struct ObjectMergeTransformer {
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ObjectMergeTransformer {
  /// Creates a new `ObjectMergeTransformer`.
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

impl Default for ObjectMergeTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ObjectMergeTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for ObjectMergeTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ObjectMergeTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ObjectMergeTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|value| {
      if let Value::Array(arr) = value {
        let mut merged = Map::new();
        for obj in arr {
          if let Some(map) = obj.as_object() {
            for (k, v) in map {
              merged.insert(k.clone(), v.clone());
            }
          }
        }
        Value::Object(merged)
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
        .unwrap_or_else(|| "object_merge_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

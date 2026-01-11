//! Object entries transformer for StreamWeave.
//!
//! This module provides [`ObjectEntriesTransformer`], a transformer that extracts
//! key-value pairs from JSON objects, producing arrays of [key, value] pairs. It
//! processes JSON objects and converts them into arrays of entry pairs, making it
//! ideal for object iteration and key-value extraction operations.
//!
//! # Overview
//!
//! [`ObjectEntriesTransformer`] is useful for extracting key-value pairs from JSON
//! objects in streaming pipelines. It processes JSON objects and converts them
//! into arrays of [key, value] pairs, similar to JavaScript's `Object.entries()`
//! function.
//!
//! # Key Concepts
//!
//! - **Entry Extraction**: Extracts key-value pairs from JSON objects
//! - **Array Output**: Produces arrays of [key, value] pairs
//! - **Object Processing**: Works with JSON objects only
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`ObjectEntriesTransformer`]**: Transformer that extracts object entries
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::ObjectEntriesTransformer;
//! use streamweave::PipelineBuilder;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that extracts object entries
//! let transformer = ObjectEntriesTransformer::new();
//!
//! // Input: [json!({"name": "John", "age": 30})]
//! // Output: [json!([["name", "John"], ["age", 30]])]
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::ObjectEntriesTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = ObjectEntriesTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("entries-extractor".to_string());
//! ```
//!
//! # Behavior
//!
//! The transformer extracts all key-value pairs from each input JSON object and
//! produces an array of [key, value] pairs. Non-object values produce empty arrays.
//!
//! # Design Decisions
//!
//! - **Object-Only**: Works with JSON objects, returns empty array for non-objects
//! - **Entry Format**: Uses [key, value] array format for each entry
//! - **Array Output**: Produces arrays of entries for easy iteration
//! - **Simple Extraction**: Focuses solely on entry extraction for clarity
//!
//! # Integration with StreamWeave
//!
//! [`ObjectEntriesTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// A transformer that extracts key-value pairs from JSON objects.
///
/// Takes each input object and outputs an array of [key, value] pairs.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::ObjectEntriesTransformer;
/// use serde_json::json;
///
/// let transformer = ObjectEntriesTransformer::new();
/// // Input: [json!({"name": "John", "age": 30})]
/// // Output: [json!([["name", "John"], ["age", 30]])]
/// ```
pub struct ObjectEntriesTransformer {
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ObjectEntriesTransformer {
  /// Creates a new `ObjectEntriesTransformer`.
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

impl Default for ObjectEntriesTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ObjectEntriesTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for ObjectEntriesTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ObjectEntriesTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ObjectEntriesTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|obj| {
      if let Some(map) = obj.as_object() {
        let entries: Vec<Value> = map
          .iter()
          .map(|(k, v)| Value::Array(vec![Value::String(k.clone()), v.clone()]))
          .collect();
        Value::Array(entries)
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
        .unwrap_or_else(|| "object_entries_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

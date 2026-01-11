//! # Object Has Property Transformer
//!
//! Transformer for filtering JSON objects based on property existence in StreamWeave pipelines.
//!
//! This module provides [`ObjectHasPropertyTransformer`], a transformer that filters
//! JSON objects based on whether they contain a specific property key. Only objects
//! that have the specified property are passed through the stream.
//!
//! # Overview
//!
//! [`ObjectHasPropertyTransformer`] is useful for filtering objects in streams based
//! on property existence. It checks if JSON objects contain a specific key and only
//! yields objects that have that property. Non-objects and objects without the property
//! are filtered out.
//!
//! # Key Concepts
//!
//! - **Property Filtering**: Filters objects based on property key existence
//! - **Key Checking**: Uses `contains_key` to check for property presence
//! - **Object-Only**: Only processes JSON objects (filters out non-objects)
//! - **Exact Match**: Checks for exact key match (case-sensitive)
//! - **Stream Filtering**: Uses stream filtering to remove non-matching objects
//!
//! # Core Types
//!
//! - **[`ObjectHasPropertyTransformer`]**: Transformer that filters objects by property existence
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::ObjectHasPropertyTransformer;
//! use serde_json::json;
//!
//! // Create a transformer that filters objects with "name" property
//! let transformer = ObjectHasPropertyTransformer::new("name");
//!
//! // Input: [json!({"name": "John", "age": 30}), json!({"age": 30})]
//! // Output: [json!({"name": "John", "age": 30})]
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::ObjectHasPropertyTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = ObjectHasPropertyTransformer::new("email")
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("has-email-filter".to_string());
//! ```
//!
//! ## Filtering Required Fields
//!
//! ```rust
//! use streamweave::transformers::ObjectHasPropertyTransformer;
//!
//! // Filter to only objects that have an "id" field
//! let transformer = ObjectHasPropertyTransformer::new("id");
//!
//! // Only objects with "id" property will pass through
//! ```
//!
//! # Design Decisions
//!
//! ## Property Existence Checking
//!
//! Uses `contains_key` to check for property existence, which is efficient and
//! doesn't require reading the property value. This makes the transformer suitable
//! for high-throughput filtering operations.
//!
//! ## Object-Only Processing
//!
//! Only JSON objects are processed. Non-object values (arrays, primitives, null) are
//! filtered out. This design ensures type safety and prevents errors when checking
//! for properties.
//!
//! ## Case-Sensitive Matching
//!
//! Property key matching is case-sensitive, matching JSON object key semantics.
//! This ensures predictable behavior and matches common JSON processing expectations.
//!
//! ## Stream Filtering
//!
//! Uses stream filtering to remove non-matching objects, which is efficient and
//! doesn't require buffering. This allows the transformer to handle large streams
//! without memory issues.
//!
//! ## No Value Checking
//!
//! Only checks for property existence, not the property value. Objects with the
//! property but `null` value will still pass through. For value-based filtering,
//! combine with other transformers.
//!
//! # Integration with StreamWeave
//!
//! [`ObjectHasPropertyTransformer`] integrates seamlessly with StreamWeave's pipeline
//! and graph systems:
//!
//! - **Pipeline API**: Use in pipelines for object filtering
//! - **Graph API**: Wrap in graph nodes for graph-based filtering
//! - **Error Handling**: Supports standard error handling strategies
//! - **Configuration**: Supports configuration via [`TransformerConfig`]
//! - **JSON Values**: Works with `serde_json::Value` for flexible JSON handling
//!
//! # Common Patterns
//!
//! ## Required Field Validation
//!
//! Filter objects to ensure they have required fields:
//!
//! ```rust
//! use streamweave::transformers::ObjectHasPropertyTransformer;
//!
//! // Ensure all objects have an "id" field
//! let transformer = ObjectHasPropertyTransformer::new("id");
//! ```
//!
//! ## Optional Field Processing
//!
//! Process only objects that have optional fields:
//!
//! ```rust
//! use streamweave::transformers::ObjectHasPropertyTransformer;
//!
//! // Only process objects with "metadata" field
//! let transformer = ObjectHasPropertyTransformer::new("metadata");
//! ```
//!
//! ## Combining with Other Filters
//!
//! Chain multiple filters for complex filtering logic:
//!
//! ```rust
//! use streamweave::transformers::ObjectHasPropertyTransformer;
//!
//! // First filter: objects with "email"
//! let has_email = ObjectHasPropertyTransformer::new("email");
//! // Second filter: objects with "phone" (chain in pipeline)
//! // let has_phone = ObjectHasPropertyTransformer::new("phone");
//! ```
//!
//! # Filtering Behavior
//!
//! ## Matching Objects
//!
//! Objects that contain the specified property key (regardless of value) are
//! passed through unchanged. The property value is not checked or modified.
//!
//! ## Non-Matching Objects
//!
//! Objects without the specified property are filtered out (not yielded).
//! Non-object values (arrays, primitives, null) are also filtered out.
//!
//! ## Performance
//!
//! Property existence checking is O(1) for hash maps (JSON objects), making
//! this transformer efficient for high-throughput filtering operations.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// A transformer that filters objects that have a specific property.
///
/// Only objects that contain the specified property key are passed through.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::ObjectHasPropertyTransformer;
///
/// let transformer = ObjectHasPropertyTransformer::new("name");
/// // Input: [{"name": "John"}, {"age": 30}]
/// // Output: [{"name": "John"}]
/// ```
pub struct ObjectHasPropertyTransformer {
  /// Property key to check for
  key: String,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ObjectHasPropertyTransformer {
  /// Creates a new `ObjectHasPropertyTransformer`.
  ///
  /// # Arguments
  ///
  /// * `key` - The property key to check for.
  pub fn new(key: impl Into<String>) -> Self {
    Self {
      key: key.into(),
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

impl Clone for ObjectHasPropertyTransformer {
  fn clone(&self) -> Self {
    Self {
      key: self.key.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for ObjectHasPropertyTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ObjectHasPropertyTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ObjectHasPropertyTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let key = self.key.clone();
    Box::pin(input.filter(move |obj| {
      let key = key.clone();
      futures::future::ready(
        obj
          .as_object()
          .map(|map| map.contains_key(&key))
          .unwrap_or(false),
      )
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
        .unwrap_or_else(|| "object_has_property_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

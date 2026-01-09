//! Object property transformer for StreamWeave
//!
//! Gets, sets, or deletes properties from JSON objects.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Property operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyOperation {
  /// Get property value
  Get,
  /// Set property value
  Set,
  /// Delete property
  Delete,
}

/// A transformer that performs property operations on JSON objects.
///
/// Supports getting, setting, or deleting properties from JSON objects.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{ObjectPropertyTransformer, PropertyOperation};
///
/// // Get property
/// let transformer = ObjectPropertyTransformer::new("name", PropertyOperation::Get, None);
/// // Input: [{"name": "John", "age": 30}]
/// // Output: ["John"]
/// ```
pub struct ObjectPropertyTransformer {
  /// Property key
  key: String,
  /// Operation to perform
  operation: PropertyOperation,
  /// Value to set (only used for Set operation)
  set_value: Option<Value>,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ObjectPropertyTransformer {
  /// Creates a new `ObjectPropertyTransformer`.
  ///
  /// # Arguments
  ///
  /// * `key` - The property key to operate on.
  /// * `operation` - The operation to perform (Get, Set, or Delete).
  /// * `set_value` - Optional value to set (required for Set operation).
  pub fn new(
    key: impl Into<String>,
    operation: PropertyOperation,
    set_value: Option<Value>,
  ) -> Self {
    Self {
      key: key.into(),
      operation,
      set_value,
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

impl Clone for ObjectPropertyTransformer {
  fn clone(&self) -> Self {
    Self {
      key: self.key.clone(),
      operation: self.operation,
      set_value: self.set_value.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for ObjectPropertyTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ObjectPropertyTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ObjectPropertyTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let key = self.key.clone();
    let operation = self.operation;
    let set_value = self.set_value.clone();
    Box::pin(input.map(move |mut obj| match operation {
      PropertyOperation::Get => obj.get(&key).cloned().unwrap_or(Value::Null),
      PropertyOperation::Set => {
        if let Some(map) = obj.as_object_mut()
          && let Some(val) = set_value.clone()
        {
          map.insert(key.clone(), val);
        }
        obj
      }
      PropertyOperation::Delete => {
        if let Some(map) = obj.as_object_mut() {
          map.remove(&key);
        }
        obj
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
        .unwrap_or_else(|| "object_property_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

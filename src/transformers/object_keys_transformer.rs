//! Object keys transformer for StreamWeave
//!
//! Extracts keys from JSON objects, producing a stream of arrays of keys.

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

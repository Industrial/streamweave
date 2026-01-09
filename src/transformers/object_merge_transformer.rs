//! Object merge transformer for StreamWeave
//!
//! Merges multiple JSON objects into a single object.

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

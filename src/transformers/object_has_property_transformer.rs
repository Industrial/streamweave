//! Object has property transformer for StreamWeave
//!
//! Filters objects that have a specific property.

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

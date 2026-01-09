//! Array contains transformer for StreamWeave
//!
//! Filters arrays that contain a specific value.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// A transformer that filters arrays that contain a specific value.
///
/// Only arrays that contain the specified value are passed through.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::ArrayContainsTransformer;
///
/// let transformer = ArrayContainsTransformer::new(serde_json::json!(3));
/// // Input: [[1, 2, 3], [4, 5]]
/// // Output: [[1, 2, 3]]
/// ```
pub struct ArrayContainsTransformer {
  /// Value to check for
  search_value: Value,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ArrayContainsTransformer {
  /// Creates a new `ArrayContainsTransformer`.
  ///
  /// # Arguments
  ///
  /// * `search_value` - The value to check for.
  pub fn new(search_value: Value) -> Self {
    Self {
      search_value,
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

impl Clone for ArrayContainsTransformer {
  fn clone(&self) -> Self {
    Self {
      search_value: self.search_value.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for ArrayContainsTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ArrayContainsTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ArrayContainsTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let search_value = self.search_value.clone();
    Box::pin(input.filter(move |value| {
      let search_value = search_value.clone();
      futures::future::ready(if let Value::Array(arr) = value {
        arr.contains(&search_value)
      } else {
        false
      })
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
        .unwrap_or_else(|| "array_contains_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

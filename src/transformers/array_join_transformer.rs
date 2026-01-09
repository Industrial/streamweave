//! Array join transformer for StreamWeave
//!
//! Joins array elements into a string using a delimiter.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// A transformer that joins array elements into a string.
///
/// Takes each input array and joins its elements with the specified delimiter.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::ArrayJoinTransformer;
///
/// let transformer = ArrayJoinTransformer::new(",");
/// // Input: [[1, 2, 3]]
/// // Output: ["1,2,3"]
/// ```
pub struct ArrayJoinTransformer {
  /// Delimiter to join with
  delimiter: String,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ArrayJoinTransformer {
  /// Creates a new `ArrayJoinTransformer` with the specified delimiter.
  ///
  /// # Arguments
  ///
  /// * `delimiter` - The delimiter to join with.
  pub fn new(delimiter: impl Into<String>) -> Self {
    Self {
      delimiter: delimiter.into(),
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

impl Clone for ArrayJoinTransformer {
  fn clone(&self) -> Self {
    Self {
      delimiter: self.delimiter.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for ArrayJoinTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ArrayJoinTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ArrayJoinTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let delimiter = self.delimiter.clone();
    Box::pin(input.map(move |value| {
      if let Value::Array(arr) = value {
        let parts: Vec<String> = arr
          .iter()
          .map(|v| match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            _ => serde_json::to_string(v).unwrap_or_else(|_| "".to_string()),
          })
          .collect();
        Value::String(parts.join(&delimiter))
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
        .unwrap_or_else(|| "array_join_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

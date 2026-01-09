//! Array index of transformer for StreamWeave
//!
//! Finds the index of a value in arrays, producing a stream of `Option<usize>` values.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Index search mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayIndexOfMode {
  /// Find first occurrence
  First,
  /// Find last occurrence
  Last,
}

/// A transformer that finds the index of a value in arrays.
///
/// Takes each input array and finds the index of the specified value,
/// producing `Option<usize>` (None if not found).
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{ArrayIndexOfTransformer, ArrayIndexOfMode};
///
/// let transformer = ArrayIndexOfTransformer::new(serde_json::json!(3), ArrayIndexOfMode::First);
/// // Input: [[1, 2, 3, 3, 4]]
/// // Output: [Some(2)]
/// ```
pub struct ArrayIndexOfTransformer {
  /// Value to find
  search_value: Value,
  /// Search mode
  mode: ArrayIndexOfMode,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ArrayIndexOfTransformer {
  /// Creates a new `ArrayIndexOfTransformer`.
  ///
  /// # Arguments
  ///
  /// * `search_value` - The value to find.
  /// * `mode` - The search mode (First or Last).
  pub fn new(search_value: Value, mode: ArrayIndexOfMode) -> Self {
    Self {
      search_value,
      mode,
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

impl Clone for ArrayIndexOfTransformer {
  fn clone(&self) -> Self {
    Self {
      search_value: self.search_value.clone(),
      mode: self.mode,
      config: self.config.clone(),
    }
  }
}

impl Input for ArrayIndexOfTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ArrayIndexOfTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ArrayIndexOfTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let search_value = self.search_value.clone();
    let mode = self.mode;
    Box::pin(input.map(move |value| {
      if let Value::Array(arr) = value {
        let index = match mode {
          ArrayIndexOfMode::First => arr.iter().position(|v| v == &search_value),
          ArrayIndexOfMode::Last => arr.iter().rposition(|v| v == &search_value),
        };
        index
          .map(|idx| Value::Number(idx.into()))
          .unwrap_or(Value::Null)
      } else {
        Value::Null
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
        .unwrap_or_else(|| "array_index_of_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

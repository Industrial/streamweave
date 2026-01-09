//! Array slice transformer for StreamWeave
//!
//! Extracts slices from arrays using start and end indices.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// A transformer that extracts slices from arrays.
///
/// Extracts a slice using start and optional end indices.
/// If end is None, extracts from start to the end of the array.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::ArraySliceTransformer;
///
/// let transformer = ArraySliceTransformer::new(1, Some(3));
/// // Input: [[1, 2, 3, 4, 5]]
/// // Output: [[2, 3]]
/// ```
pub struct ArraySliceTransformer {
  /// Start index (inclusive)
  start: usize,
  /// End index (exclusive), None means to end of array
  end: Option<usize>,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ArraySliceTransformer {
  /// Creates a new `ArraySliceTransformer`.
  ///
  /// # Arguments
  ///
  /// * `start` - Start index (inclusive).
  /// * `end` - End index (exclusive), or None to extract to end of array.
  pub fn new(start: usize, end: Option<usize>) -> Self {
    Self {
      start,
      end,
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

impl Clone for ArraySliceTransformer {
  fn clone(&self) -> Self {
    Self {
      start: self.start,
      end: self.end,
      config: self.config.clone(),
    }
  }
}

impl Input for ArraySliceTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ArraySliceTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ArraySliceTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let start = self.start;
    let end = self.end;
    Box::pin(input.map(move |value| {
      if let Value::Array(arr) = value {
        let len = arr.len();
        let start_idx = start.min(len);
        let end_idx = end.map(|e| e.min(len)).unwrap_or(len);
        if start_idx >= end_idx {
          Value::Array(vec![])
        } else {
          Value::Array(arr[start_idx..end_idx].to_vec())
        }
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
        .unwrap_or_else(|| "array_slice_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

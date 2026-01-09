//! Math random transformer for StreamWeave
//!
//! Generates random numbers.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use rand::Rng;
use serde_json::Value;
use std::pin::Pin;

/// A transformer that generates random numbers.
///
/// Generates random f64 values in the specified range [min, max).
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::MathRandomTransformer;
///
/// let transformer = MathRandomTransformer::new(0.0, 1.0);
/// // Input: [(), (), ()]
/// // Output: [0.123, 0.456, 0.789] (random values)
/// ```
pub struct MathRandomTransformer {
  /// Minimum value (inclusive)
  min: f64,
  /// Maximum value (exclusive)
  max: f64,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl MathRandomTransformer {
  /// Creates a new `MathRandomTransformer`.
  ///
  /// # Arguments
  ///
  /// * `min` - Minimum value (inclusive).
  /// * `max` - Maximum value (exclusive).
  pub fn new(min: f64, max: f64) -> Self {
    Self {
      min,
      max,
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

impl Clone for MathRandomTransformer {
  fn clone(&self) -> Self {
    Self {
      min: self.min,
      max: self.max,
      config: self.config.clone(),
    }
  }
}

impl Input for MathRandomTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathRandomTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathRandomTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let min = self.min;
    let max = self.max;
    Box::pin(input.map(move |_value| {
      let mut rng = rand::thread_rng();
      let result = rng.gen_range(min..max);
      serde_json::Number::from_f64(result)
        .map(Value::Number)
        .unwrap_or(Value::Null)
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
        .unwrap_or_else(|| "math_random_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

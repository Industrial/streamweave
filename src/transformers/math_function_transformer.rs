//! Math function transformer for StreamWeave
//!
//! Performs power and root operations (Power, SquareRoot, Cbrt) on numeric values.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Math function type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathFunction {
  /// Power (x^y)
  Power,
  /// Square root
  SquareRoot,
  /// Cube root
  Cbrt,
}

/// A transformer that performs power and root operations on numeric values.
///
/// Supports Power, SquareRoot, and Cbrt operations.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{MathFunctionTransformer, MathFunction};
///
/// let transformer = MathFunctionTransformer::new(MathFunction::SquareRoot, None);
/// // Input: [4, 9, 16]
/// // Output: [2, 3, 4]
/// ```
pub struct MathFunctionTransformer {
  /// Function to perform
  function: MathFunction,
  /// Exponent for Power operation (None for SquareRoot/Cbrt)
  exponent: Option<f64>,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl MathFunctionTransformer {
  /// Creates a new `MathFunctionTransformer`.
  ///
  /// # Arguments
  ///
  /// * `function` - The function to perform.
  /// * `exponent` - Optional exponent for Power operation (required for Power, ignored for others).
  pub fn new(function: MathFunction, exponent: Option<f64>) -> Self {
    Self {
      function,
      exponent,
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

impl Clone for MathFunctionTransformer {
  fn clone(&self) -> Self {
    Self {
      function: self.function,
      exponent: self.exponent,
      config: self.config.clone(),
    }
  }
}

impl Input for MathFunctionTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathFunctionTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathFunctionTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let function = self.function;
    let exponent = self.exponent;
    Box::pin(input.map(move |value| {
      let num = match value {
        Value::Number(n) => n.as_f64(),
        _ => None,
      };
      if let Some(num) = num {
        let result = match function {
          MathFunction::Power => {
            if let Some(exp) = exponent {
              num.powf(exp)
            } else {
              return Value::Null;
            }
          }
          MathFunction::SquareRoot => {
            if num < 0.0 {
              return Value::Null;
            }
            num.sqrt()
          }
          MathFunction::Cbrt => num.cbrt(),
        };
        serde_json::Number::from_f64(result)
          .map(Value::Number)
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
        .unwrap_or_else(|| "math_function_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

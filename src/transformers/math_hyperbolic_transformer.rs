//! Math hyperbolic transformer for StreamWeave
//!
//! Performs hyperbolic operations (Sinh, Cosh, Tanh, Asinh, Acosh, Atanh) on numeric values.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Hyperbolic function type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyperbolicFunction {
  /// Hyperbolic sine
  Sinh,
  /// Hyperbolic cosine
  Cosh,
  /// Hyperbolic tangent
  Tanh,
  /// Inverse hyperbolic sine
  Asinh,
  /// Inverse hyperbolic cosine
  Acosh,
  /// Inverse hyperbolic tangent
  Atanh,
}

/// A transformer that performs hyperbolic operations.
///
/// Supports Sinh, Cosh, Tanh, Asinh, Acosh, and Atanh operations.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{MathHyperbolicTransformer, HyperbolicFunction};
///
/// let transformer = MathHyperbolicTransformer::new(HyperbolicFunction::Sinh);
/// // Input: [0, 1]
/// // Output: [0, 1.1752...]
/// ```
pub struct MathHyperbolicTransformer {
  /// Function to perform
  function: HyperbolicFunction,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl MathHyperbolicTransformer {
  /// Creates a new `MathHyperbolicTransformer`.
  ///
  /// # Arguments
  ///
  /// * `function` - The function to perform.
  pub fn new(function: HyperbolicFunction) -> Self {
    Self {
      function,
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

impl Clone for MathHyperbolicTransformer {
  fn clone(&self) -> Self {
    Self {
      function: self.function,
      config: self.config.clone(),
    }
  }
}

impl Input for MathHyperbolicTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathHyperbolicTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathHyperbolicTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let function = self.function;
    Box::pin(input.map(move |value| {
      let num = match value {
        Value::Number(n) => n.as_f64(),
        _ => None,
      };
      if let Some(num) = num {
        let result = match function {
          HyperbolicFunction::Sinh => num.sinh(),
          HyperbolicFunction::Cosh => num.cosh(),
          HyperbolicFunction::Tanh => num.tanh(),
          HyperbolicFunction::Asinh => num.asinh(),
          HyperbolicFunction::Acosh => {
            if num < 1.0 {
              return Value::Null;
            }
            num.acosh()
          }
          HyperbolicFunction::Atanh => {
            if num <= -1.0 || num >= 1.0 {
              return Value::Null;
            }
            num.atanh()
          }
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
        .unwrap_or_else(|| "math_hyperbolic_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

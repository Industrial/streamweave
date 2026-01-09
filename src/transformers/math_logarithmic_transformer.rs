//! Math logarithmic transformer for StreamWeave
//!
//! Performs logarithmic and exponential operations on numeric values.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Logarithmic function type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogarithmicFunction {
  /// Natural logarithm (ln)
  Log,
  /// Base-10 logarithm
  Log10,
  /// Base-2 logarithm
  Log2,
  /// Log(1+x)
  Log1p,
  /// Exponential (e^x)
  Exp,
  /// Exp(x)-1
  Expm1,
}

/// A transformer that performs logarithmic and exponential operations.
///
/// Supports Log, Log10, Log2, Log1p, Exp, and Expm1 operations.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{MathLogarithmicTransformer, LogarithmicFunction};
///
/// let transformer = MathLogarithmicTransformer::new(LogarithmicFunction::Log);
/// // Input: [1, 2.71828] (1, e)
/// // Output: [0, 1]
/// ```
pub struct MathLogarithmicTransformer {
  /// Function to perform
  function: LogarithmicFunction,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl MathLogarithmicTransformer {
  /// Creates a new `MathLogarithmicTransformer`.
  ///
  /// # Arguments
  ///
  /// * `function` - The function to perform.
  pub fn new(function: LogarithmicFunction) -> Self {
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

impl Clone for MathLogarithmicTransformer {
  fn clone(&self) -> Self {
    Self {
      function: self.function,
      config: self.config.clone(),
    }
  }
}

impl Input for MathLogarithmicTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathLogarithmicTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathLogarithmicTransformer {
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
          LogarithmicFunction::Log => {
            if num <= 0.0 {
              return Value::Null;
            }
            num.ln()
          }
          LogarithmicFunction::Log10 => {
            if num <= 0.0 {
              return Value::Null;
            }
            num.log10()
          }
          LogarithmicFunction::Log2 => {
            if num <= 0.0 {
              return Value::Null;
            }
            num.log2()
          }
          LogarithmicFunction::Log1p => {
            if num <= -1.0 {
              return Value::Null;
            }
            num.ln_1p()
          }
          LogarithmicFunction::Exp => num.exp(),
          LogarithmicFunction::Expm1 => num.exp_m1(),
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
        .unwrap_or_else(|| "math_logarithmic_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

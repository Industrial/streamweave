//! Math operation transformer for StreamWeave
//!
//! Performs arithmetic operations (Add, Subtract, Multiply, Divide, Modulo) on numeric values.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Math operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathOperation {
  /// Addition
  Add,
  /// Subtraction
  Subtract,
  /// Multiplication
  Multiply,
  /// Division
  Divide,
  /// Modulo
  Modulo,
}

/// A transformer that performs arithmetic operations on numeric values.
///
/// Supports Add, Subtract, Multiply, Divide, and Modulo operations.
/// Works with f64 values extracted from JSON numbers.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{MathOperationTransformer, MathOperation};
///
/// let transformer = MathOperationTransformer::new(MathOperation::Add, 10.0);
/// // Input: [5, 3, 7]
/// // Output: [15, 13, 17]
/// ```
pub struct MathOperationTransformer {
  /// Operation to perform
  operation: MathOperation,
  /// Operand value
  operand: f64,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl MathOperationTransformer {
  /// Creates a new `MathOperationTransformer`.
  ///
  /// # Arguments
  ///
  /// * `operation` - The operation to perform.
  /// * `operand` - The operand value to use in the operation.
  pub fn new(operation: MathOperation, operand: f64) -> Self {
    Self {
      operation,
      operand,
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

impl Clone for MathOperationTransformer {
  fn clone(&self) -> Self {
    Self {
      operation: self.operation,
      operand: self.operand,
      config: self.config.clone(),
    }
  }
}

impl Input for MathOperationTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathOperationTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathOperationTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let operation = self.operation;
    let operand = self.operand;
    Box::pin(input.map(move |value| {
      let num = match value {
        Value::Number(n) => n.as_f64(),
        _ => None,
      };
      if let Some(num) = num {
        let result = match operation {
          MathOperation::Add => num + operand,
          MathOperation::Subtract => num - operand,
          MathOperation::Multiply => num * operand,
          MathOperation::Divide => {
            if operand == 0.0 {
              return Value::Null;
            }
            num / operand
          }
          MathOperation::Modulo => {
            if operand == 0.0 {
              return Value::Null;
            }
            num % operand
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
        .unwrap_or_else(|| "math_operation_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

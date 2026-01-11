//! Math trigonometric transformer for performing trigonometric operations.
//!
//! This module provides [`MathTrigonometricTransformer`] and [`TrigonometricFunction`],
//! types for performing trigonometric operations on numeric values in StreamWeave
//! pipelines. It supports Sin, Cos, Tan, Asin, Acos, Atan, and Atan2 operations,
//! making it ideal for mathematical computations. It implements the [`Transformer`]
//! trait for use in StreamWeave pipelines and graphs.
//!
//! # Overview
//!
//! [`MathTrigonometricTransformer`] is useful for performing trigonometric operations
//! on numeric values in StreamWeave pipelines. It processes JSON numeric values and
//! applies trigonometric functions, making it ideal for mathematical computations.
//!
//! # Key Concepts
//!
//! - **Trigonometric Functions**: Supports Sin, Cos, Tan and their inverses
//! - **Atan2 Support**: Supports Atan2 with optional second operand
//! - **JSON Value Support**: Works with `serde_json::Value` numeric types
//! - **Transformer Trait**: Implements `Transformer` for pipeline integration
//!
//! # Core Types
//!
//! - **[`MathTrigonometricTransformer`]**: Transformer that performs trigonometric operations
//! - **[`TrigonometricFunction`]**: Enum representing different trigonometric functions
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::{MathTrigonometricTransformer, TrigonometricFunction};
//!
//! // Calculate sine
//! let transformer = MathTrigonometricTransformer::new(TrigonometricFunction::Sin, None);
//! ```
//!
//! ## Atan2
//!
//! ```rust
//! use streamweave::transformers::{MathTrigonometricTransformer, TrigonometricFunction};
//!
//! // Calculate arctangent2
//! let transformer = MathTrigonometricTransformer::new(TrigonometricFunction::Atan2, Some(1.0));
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::{MathTrigonometricTransformer, TrigonometricFunction};
//! use streamweave::ErrorStrategy;
//!
//! // Create a trigonometric transformer with error handling
//! let transformer = MathTrigonometricTransformer::new(TrigonometricFunction::Sin, None)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("sin".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible numeric
//!   value handling
//! - **Function Enum**: Uses enum-based function selection for type safety
//! - **Optional Second Operand**: Supports optional second operand for Atan2
//! - **Transformer Trait**: Implements `Transformer` for integration with
//!   pipeline system
//!
//! # Integration with StreamWeave
//!
//! [`MathTrigonometricTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Trigonometric function type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrigonometricFunction {
  /// Sine
  Sin,
  /// Cosine
  Cos,
  /// Tangent
  Tan,
  /// Arc sine
  Asin,
  /// Arc cosine
  Acos,
  /// Arc tangent
  Atan,
  /// Two-argument arc tangent (requires second operand)
  Atan2,
}

/// A transformer that performs trigonometric operations.
///
/// Supports Sin, Cos, Tan, Asin, Acos, Atan, and Atan2 operations.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{MathTrigonometricTransformer, TrigonometricFunction};
///
/// let transformer = MathTrigonometricTransformer::new(TrigonometricFunction::Sin, None);
/// // Input: [0, 1.5708] (0, π/2)
/// // Output: [0, 1]
/// ```
pub struct MathTrigonometricTransformer {
  /// Function to perform
  function: TrigonometricFunction,
  /// Second operand for Atan2 (None for other functions)
  second_operand: Option<f64>,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl MathTrigonometricTransformer {
  /// Creates a new `MathTrigonometricTransformer`.
  ///
  /// # Arguments
  ///
  /// * `function` - The function to perform.
  /// * `second_operand` - Optional second operand for Atan2 (required for Atan2, ignored for others).
  pub fn new(function: TrigonometricFunction, second_operand: Option<f64>) -> Self {
    Self {
      function,
      second_operand,
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

impl Clone for MathTrigonometricTransformer {
  fn clone(&self) -> Self {
    Self {
      function: self.function,
      second_operand: self.second_operand,
      config: self.config.clone(),
    }
  }
}

impl Input for MathTrigonometricTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathTrigonometricTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathTrigonometricTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let function = self.function;
    let second_operand = self.second_operand;
    Box::pin(input.map(move |value| {
      let num = match value {
        Value::Number(n) => n.as_f64(),
        _ => None,
      };
      if let Some(num) = num {
        let result = match function {
          TrigonometricFunction::Sin => num.sin(),
          TrigonometricFunction::Cos => num.cos(),
          TrigonometricFunction::Tan => num.tan(),
          TrigonometricFunction::Asin => {
            if !(-1.0..=1.0).contains(&num) {
              return Value::Null;
            }
            num.asin()
          }
          TrigonometricFunction::Acos => {
            if !(-1.0..=1.0).contains(&num) {
              return Value::Null;
            }
            num.acos()
          }
          TrigonometricFunction::Atan => num.atan(),
          TrigonometricFunction::Atan2 => {
            if let Some(y) = second_operand {
              num.atan2(y)
            } else {
              return Value::Null;
            }
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
        .unwrap_or_else(|| "math_trigonometric_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

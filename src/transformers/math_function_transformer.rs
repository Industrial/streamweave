//! Math function transformer for performing power and root operations.
//!
//! This module provides [`MathFunctionTransformer`] and [`MathFunction`], types for
//! performing power and root operations on numeric values in StreamWeave pipelines.
//! It supports Power (x^y), SquareRoot, and CubeRoot (Cbrt) operations, making it
//! ideal for mathematical computations. It implements the [`Transformer`] trait for
//! use in StreamWeave pipelines and graphs.
//!
//! # Overview
//!
//! [`MathFunctionTransformer`] is useful for performing power and root operations
//! on numeric values in StreamWeave pipelines. It processes JSON numeric values
//! and applies power/root functions, making it ideal for mathematical computations.
//!
//! # Key Concepts
//!
//! - **Power Operations**: Supports Power (x^y) with configurable exponent
//! - **Root Operations**: Supports SquareRoot and CubeRoot operations
//! - **JSON Value Support**: Works with `serde_json::Value` numeric types
//! - **Transformer Trait**: Implements `Transformer` for pipeline integration
//!
//! # Core Types
//!
//! - **[`MathFunctionTransformer`]**: Transformer that performs power and root operations
//! - **[`MathFunction`]**: Enum representing different math functions (Power, SquareRoot, Cbrt)
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::{MathFunctionTransformer, MathFunction};
//!
//! // Square root
//! let transformer = MathFunctionTransformer::new(MathFunction::SquareRoot, None);
//! ```
//!
//! ## Power Operation
//!
//! ```rust
//! use streamweave::transformers::{MathFunctionTransformer, MathFunction};
//!
//! // Raise to power of 3
//! let transformer = MathFunctionTransformer::new(MathFunction::Power, Some(3.0));
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::{MathFunctionTransformer, MathFunction};
//! use streamweave::ErrorStrategy;
//!
//! // Create a math function transformer with error handling
//! let transformer = MathFunctionTransformer::new(MathFunction::SquareRoot, None)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("sqrt".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible numeric
//!   value handling
//! - **Function Enum**: Uses enum-based function selection for type safety
//! - **Optional Operand**: Supports optional exponent for Power operation
//! - **Transformer Trait**: Implements `Transformer` for integration with
//!   pipeline system
//!
//! # Integration with StreamWeave
//!
//! [`MathFunctionTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

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

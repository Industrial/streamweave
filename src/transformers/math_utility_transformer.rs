//! Math utility transformer for performing utility math operations.
//!
//! This module provides [`MathUtilityTransformer`] and [`MathUtilityFunction`], types
//! for performing utility math operations on numeric values in StreamWeave pipelines.
//! It supports Hypot, Imul, Sign, Clz32, and Fround operations, making it ideal for
//! mathematical computations and conversions. It implements the [`Transformer`] trait
//! for use in StreamWeave pipelines and graphs.
//!
//! # Overview
//!
//! [`MathUtilityTransformer`] is useful for performing utility mathematical operations
//! on numeric values in StreamWeave pipelines. It processes JSON numeric values and
//! applies utility functions, making it ideal for mathematical computations and
//! conversions.
//!
//! # Key Concepts
//!
//! - **Utility Operations**: Supports Hypot, Imul, Sign, Clz32, Fround
//! - **Binary Operations**: Some operations (Hypot, Imul) support optional second operand
//! - **JSON Value Support**: Works with `serde_json::Value` numeric types
//! - **Transformer Trait**: Implements `Transformer` for pipeline integration
//!
//! # Core Types
//!
//! - **[`MathUtilityTransformer`]**: Transformer that performs utility math operations
//! - **[`MathUtilityFunction`]**: Enum representing different utility functions
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::{MathUtilityTransformer, MathUtilityFunction};
//!
//! // Calculate sign
//! let transformer = MathUtilityTransformer::new(MathUtilityFunction::Sign, None);
//! ```
//!
//! ## Hypot Operation
//!
//! ```rust
//! use streamweave::transformers::{MathUtilityTransformer, MathUtilityFunction};
//!
//! // Calculate hypotenuse
//! let transformer = MathUtilityTransformer::new(MathUtilityFunction::Hypot, Some(3.0));
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::{MathUtilityTransformer, MathUtilityFunction};
//! use streamweave::ErrorStrategy;
//!
//! // Create a utility transformer with error handling
//! let transformer = MathUtilityTransformer::new(MathUtilityFunction::Sign, None)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("sign".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible numeric
//!   value handling
//! - **Function Enum**: Uses enum-based function selection for type safety
//! - **Optional Second Operand**: Supports optional second operand for binary operations
//! - **Transformer Trait**: Implements `Transformer` for integration with
//!   pipeline system
//!
//! # Integration with StreamWeave
//!
//! [`MathUtilityTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Utility math function type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathUtilityFunction {
  /// Sign function (-1, 0, or 1)
  Sign,
  /// Hypotenuse (sqrt(x^2 + y^2))
  Hypot,
  /// Count leading zeros in 32-bit representation
  Clz32,
  /// Round to nearest 32-bit float
  Fround,
  /// 32-bit integer multiplication
  Imul,
}

/// A transformer that performs utility math operations.
///
/// Supports Sign, Hypot, Clz32, Fround, and Imul operations.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{MathUtilityTransformer, MathUtilityFunction};
///
/// let transformer = MathUtilityTransformer::new(MathUtilityFunction::Sign, None);
/// // Input: [-5, 0, 5]
/// // Output: [-1, 0, 1]
/// ```
pub struct MathUtilityTransformer {
  /// Function to perform
  function: MathUtilityFunction,
  /// Second operand for Hypot/Imul (None for other functions)
  second_operand: Option<f64>,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl MathUtilityTransformer {
  /// Creates a new `MathUtilityTransformer`.
  ///
  /// # Arguments
  ///
  /// * `function` - The function to perform.
  /// * `second_operand` - Optional second operand for Hypot/Imul (required for these, ignored for others).
  pub fn new(function: MathUtilityFunction, second_operand: Option<f64>) -> Self {
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

impl Clone for MathUtilityTransformer {
  fn clone(&self) -> Self {
    Self {
      function: self.function,
      second_operand: self.second_operand,
      config: self.config.clone(),
    }
  }
}

impl Input for MathUtilityTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathUtilityTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathUtilityTransformer {
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
          MathUtilityFunction::Sign => {
            if num > 0.0 {
              1.0
            } else if num < 0.0 {
              -1.0
            } else {
              0.0
            }
          }
          MathUtilityFunction::Hypot => {
            if let Some(y) = second_operand {
              num.hypot(y)
            } else {
              return Value::Null;
            }
          }
          MathUtilityFunction::Clz32 => {
            let n = num as u32;
            n.leading_zeros() as f64
          }
          MathUtilityFunction::Fround => {
            // Round to nearest 32-bit float representation
            (num as f32) as f64
          }
          MathUtilityFunction::Imul => {
            if let Some(y) = second_operand {
              // 32-bit integer multiplication (JavaScript-style)
              let a = num as i32;
              let b = y as i32;
              (a.wrapping_mul(b)) as f64
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
        .unwrap_or_else(|| "math_utility_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

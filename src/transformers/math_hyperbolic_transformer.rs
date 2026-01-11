//! # Math Hyperbolic Transformer
//!
//! Transformer for performing hyperbolic mathematical functions in StreamWeave pipelines.
//!
//! This module provides [`MathHyperbolicTransformer`], a transformer that performs
//! hyperbolic functions (sinh, cosh, tanh, asinh, acosh, atanh) on numeric values
//! in streams. It supports both forward hyperbolic functions and inverse hyperbolic
//! functions.
//!
//! # Overview
//!
//! [`MathHyperbolicTransformer`] applies hyperbolic mathematical functions to numeric
//! values in streams. Hyperbolic functions are analogs of trigonometric functions but
//! based on hyperbolas rather than circles. They're useful in various mathematical
//! and scientific computing applications.
//!
//! # Key Concepts
//!
//! - **Hyperbolic Functions**: Forward hyperbolic functions (sinh, cosh, tanh)
//! - **Inverse Hyperbolic Functions**: Inverse hyperbolic functions (asinh, acosh, atanh)
//! - **Numeric Processing**: Works with numeric JSON values (converted to f64)
//! - **Error Handling**: Configurable error strategies for invalid inputs
//! - **Null Output**: Returns `Value::Null` for non-numeric or invalid inputs
//!
//! # Core Types
//!
//! - **[`MathHyperbolicTransformer`]**: Transformer that performs hyperbolic functions
//! - **[`HyperbolicFunction`]**: Enum representing available hyperbolic functions
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::{MathHyperbolicTransformer, HyperbolicFunction};
//!
//! // Create a transformer that computes hyperbolic sine
//! let transformer = MathHyperbolicTransformer::new(HyperbolicFunction::Sinh);
//! // Input: [0, 1, 2]
//! // Output: [0.0, 1.175..., 3.626...]
//! ```
//!
//! ## Inverse Hyperbolic Functions
//!
//! ```rust
//! use streamweave::transformers::{MathHyperbolicTransformer, HyperbolicFunction};
//!
//! // Create a transformer that computes inverse hyperbolic sine
//! let transformer = MathHyperbolicTransformer::new(HyperbolicFunction::Asinh);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::{MathHyperbolicTransformer, HyperbolicFunction};
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = MathHyperbolicTransformer::new(HyperbolicFunction::Cosh)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("hyperbolic-cosine".to_string());
//! ```
//!
//! # Supported Functions
//!
//! ## Forward Hyperbolic Functions
//!
//! - **Sinh**: Hyperbolic sine (sinh(x))
//! - **Cosh**: Hyperbolic cosine (cosh(x))
//! - **Tanh**: Hyperbolic tangent (tanh(x))
//!
//! ## Inverse Hyperbolic Functions
//!
//! - **Asinh**: Inverse hyperbolic sine (asinh(x))
//! - **Acosh**: Inverse hyperbolic cosine (acosh(x), requires x >= 1)
//! - **Atanh**: Inverse hyperbolic tangent (atanh(x), requires |x| < 1)
//!
//! # Design Decisions
//!
//! ## Numeric Type Conversion
//!
//! All numeric JSON values are converted to `f64` for computation. This ensures
//! compatibility with floating-point mathematical operations while preserving
//! precision where possible.
//!
//! ## Invalid Input Handling
//!
//! Non-numeric values and invalid inputs (e.g., acosh with x < 1, atanh with |x| >= 1)
//! result in `Value::Null` output. This design prevents panics and allows pipelines
//! to handle invalid data gracefully.
//!
//! ## Function Enum Design
//!
//! Uses an enum to represent functions for type safety and clarity. This makes the
//! transformer easy to use and prevents invalid function selections at compile time.
//!
//! ## Single-Operation Design
//!
//! Each transformer instance performs one hyperbolic function. This design keeps
//! transformers focused and allows for clear configuration. Multiple functions can
//! be applied by chaining transformers.
//!
//! # Integration with StreamWeave
//!
//! [`MathHyperbolicTransformer`] integrates seamlessly with StreamWeave's pipeline
//! and graph systems:
//!
//! - **Pipeline API**: Use in pipelines for mathematical transformations
//! - **Graph API**: Wrap in graph nodes for graph-based mathematical processing
//! - **Error Handling**: Supports standard error handling strategies
//! - **Configuration**: Supports configuration via [`TransformerConfig`]
//! - **JSON Values**: Works with `serde_json::Value` for flexible data handling
//!
//! # Common Patterns
//!
//! ## Computing Hyperbolic Functions
//!
//! Apply hyperbolic functions to numeric streams:
//!
//! ```rust
//! use streamweave::transformers::{MathHyperbolicTransformer, HyperbolicFunction};
//!
//! // Compute hyperbolic sine for all values
//! let transformer = MathHyperbolicTransformer::new(HyperbolicFunction::Sinh);
//! ```
//!
//! ## Combining with Other Math Transformers
//!
//! Chain with other mathematical transformers for complex calculations:
//!
//! ```rust
//! use streamweave::transformers::{MathHyperbolicTransformer, MathFunctionTransformer};
//! use streamweave::transformers::{HyperbolicFunction, MathFunction};
//!
//! // First compute hyperbolic cosine, then square root
//! let cosh = MathHyperbolicTransformer::new(HyperbolicFunction::Cosh);
//! let sqrt = MathFunctionTransformer::new(MathFunction::SquareRoot, None);
//! // Chain: cosh -> sqrt
//! ```
//!
//! ## Domain Validation
//!
//! Handle domain restrictions for inverse functions:
//!
//! ```rust
//! use streamweave::transformers::{MathHyperbolicTransformer, HyperbolicFunction};
//!
//! // Acosh requires x >= 1, Atanh requires |x| < 1
//! // Invalid inputs will produce Value::Null
//! let transformer = MathHyperbolicTransformer::new(HyperbolicFunction::Acosh);
//! ```

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

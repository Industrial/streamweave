//! Math min/max transformer for finding minimum or maximum values.
//!
//! This module provides [`MathMinMaxTransformer`] and [`MinMaxOperation`], types for
//! finding minimum or maximum values in StreamWeave pipelines. It supports both Min
//! and Max operations, optionally comparing against a reference value. It implements
//! the [`Transformer`] trait for use in StreamWeave pipelines and graphs.
//!
//! # Overview
//!
//! [`MathMinMaxTransformer`] is useful for finding minimum or maximum values in
//! StreamWeave pipelines. It processes JSON numeric values (arrays or single numbers)
//! and finds the minimum or maximum, optionally comparing against a reference value,
//! making it ideal for mathematical computations and filtering.
//!
//! # Key Concepts
//!
//! - **Min/Max Operations**: Supports finding minimum or maximum values
//! - **Array Mode**: Finds min/max in arrays when no reference is provided
//! - **Comparison Mode**: Compares against a reference value when provided
//! - **JSON Value Support**: Works with `serde_json::Value` numeric types
//!
//! # Core Types
//!
//! - **[`MathMinMaxTransformer`]**: Transformer that finds minimum or maximum values
//! - **[`MinMaxOperation`]**: Enum representing Min or Max operations
//!
//! # Quick Start
//!
//! ## Basic Usage (Array Mode)
//!
//! ```rust
//! use streamweave::transformers::{MathMinMaxTransformer, MinMaxOperation};
//!
//! // Find maximum value in arrays
//! let transformer = MathMinMaxTransformer::new(MinMaxOperation::Max, None);
//! ```
//!
//! ## Comparison Mode
//!
//! ```rust
//! use streamweave::transformers::{MathMinMaxTransformer, MinMaxOperation};
//!
//! // Find minimum compared to 10
//! let transformer = MathMinMaxTransformer::new(MinMaxOperation::Min, Some(10.0));
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::{MathMinMaxTransformer, MinMaxOperation};
//! use streamweave::ErrorStrategy;
//!
//! // Create a min/max transformer with error handling
//! let transformer = MathMinMaxTransformer::new(MinMaxOperation::Max, None)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("max-finder".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible numeric
//!   value handling
//! - **Operation Enum**: Uses enum-based operation selection for type safety
//! - **Optional Reference**: Supports optional reference value for comparison
//! - **Transformer Trait**: Implements `Transformer` for integration with
//!   pipeline system
//!
//! # Integration with StreamWeave
//!
//! [`MathMinMaxTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Min/Max operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinMaxOperation {
  /// Find minimum
  Min,
  /// Find maximum
  Max,
}

/// A transformer that finds minimum or maximum values.
///
/// If input is an array, finds min/max in the array.
/// If input is a number, compares with a reference value.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{MathMinMaxTransformer, MinMaxOperation};
///
/// let transformer = MathMinMaxTransformer::new(MinMaxOperation::Max, None);
/// // Input: [[1, 5, 3, 9, 2]]
/// // Output: [9]
/// ```
pub struct MathMinMaxTransformer {
  /// Operation to perform
  operation: MinMaxOperation,
  /// Reference value for single number comparison (None for array mode)
  reference: Option<f64>,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl MathMinMaxTransformer {
  /// Creates a new `MathMinMaxTransformer`.
  ///
  /// # Arguments
  ///
  /// * `operation` - The operation to perform (Min or Max).
  /// * `reference` - Optional reference value for single number comparison (None for array mode).
  pub fn new(operation: MinMaxOperation, reference: Option<f64>) -> Self {
    Self {
      operation,
      reference,
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

impl Clone for MathMinMaxTransformer {
  fn clone(&self) -> Self {
    Self {
      operation: self.operation,
      reference: self.reference,
      config: self.config.clone(),
    }
  }
}

impl Input for MathMinMaxTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathMinMaxTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathMinMaxTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let operation = self.operation;
    let reference = self.reference;
    Box::pin(input.map(move |value| match value {
      Value::Array(arr) => {
        let numbers: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
        if numbers.is_empty() {
          Value::Null
        } else {
          let result = match operation {
            MinMaxOperation::Min => numbers.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
            MinMaxOperation::Max => numbers.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
          };
          serde_json::Number::from_f64(result)
            .map(Value::Number)
            .unwrap_or(Value::Null)
        }
      }
      Value::Number(ref n) => {
        if let (Some(num), Some(ref_val)) = (n.as_f64(), reference) {
          let result = match operation {
            MinMaxOperation::Min => num.min(ref_val),
            MinMaxOperation::Max => num.max(ref_val),
          };
          serde_json::Number::from_f64(result)
            .map(Value::Number)
            .unwrap_or(Value::Null)
        } else {
          value
        }
      }
      _ => Value::Null,
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
        .unwrap_or_else(|| "math_min_max_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

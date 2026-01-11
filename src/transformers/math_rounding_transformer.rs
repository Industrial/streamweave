//! Math rounding transformer for performing rounding and absolute value operations.
//!
//! This module provides [`MathRoundingTransformer`] and [`RoundingOperation`], types
//! for performing rounding and absolute value operations on numeric values in
//! StreamWeave pipelines. It supports Round, Floor, Ceil, Trunc, and Abs operations,
//! making it ideal for mathematical computations. It implements the [`Transformer`]
//! trait for use in StreamWeave pipelines and graphs.
//!
//! # Overview
//!
//! [`MathRoundingTransformer`] is useful for performing rounding and absolute value
//! operations on numeric values in StreamWeave pipelines. It processes JSON numeric
//! values and applies rounding/absolute functions, making it ideal for mathematical
//! computations.
//!
//! # Key Concepts
//!
//! - **Rounding Operations**: Supports Round, Floor, Ceil, Trunc
//! - **Absolute Value**: Supports Abs operation
//! - **JSON Value Support**: Works with `serde_json::Value` numeric types
//! - **Transformer Trait**: Implements `Transformer` for pipeline integration
//!
//! # Core Types
//!
//! - **[`MathRoundingTransformer`]**: Transformer that performs rounding and absolute value operations
//! - **[`RoundingOperation`]**: Enum representing different rounding operations
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::{MathRoundingTransformer, RoundingOperation};
//!
//! // Round to nearest integer
//! let transformer = MathRoundingTransformer::new(RoundingOperation::Round);
//! ```
//!
//! ## Different Operations
//!
//! ```rust
//! use streamweave::transformers::{MathRoundingTransformer, RoundingOperation};
//!
//! // Floor (round down)
//! let floor = MathRoundingTransformer::new(RoundingOperation::Floor);
//!
//! // Absolute value
//! let abs = MathRoundingTransformer::new(RoundingOperation::Absolute);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::{MathRoundingTransformer, RoundingOperation};
//! use streamweave::ErrorStrategy;
//!
//! // Create a rounding transformer with error handling
//! let transformer = MathRoundingTransformer::new(RoundingOperation::Round)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("rounder".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible numeric
//!   value handling
//! - **Operation Enum**: Uses enum-based operation selection for type safety
//! - **Transformer Trait**: Implements `Transformer` for integration with
//!   pipeline system
//!
//! # Integration with StreamWeave
//!
//! [`MathRoundingTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Rounding operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingOperation {
  /// Absolute value
  Absolute,
  /// Round to nearest integer
  Round,
  /// Ceiling (round up)
  Ceil,
  /// Floor (round down)
  Floor,
  /// Truncate (remove fractional part)
  Trunc,
}

/// A transformer that performs rounding and absolute value operations.
///
/// Supports Absolute, Round, Ceil, Floor, and Trunc operations.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{MathRoundingTransformer, RoundingOperation};
///
/// let transformer = MathRoundingTransformer::new(RoundingOperation::Round);
/// // Input: [3.7, -2.3, 5.5]
/// // Output: [4, -2, 6]
/// ```
pub struct MathRoundingTransformer {
  /// Operation to perform
  operation: RoundingOperation,
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl MathRoundingTransformer {
  /// Creates a new `MathRoundingTransformer`.
  ///
  /// # Arguments
  ///
  /// * `operation` - The operation to perform.
  pub fn new(operation: RoundingOperation) -> Self {
    Self {
      operation,
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

impl Clone for MathRoundingTransformer {
  fn clone(&self) -> Self {
    Self {
      operation: self.operation,
      config: self.config.clone(),
    }
  }
}

impl Input for MathRoundingTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathRoundingTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathRoundingTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let operation = self.operation;
    Box::pin(input.map(move |value| {
      let num = match value {
        Value::Number(n) => n.as_f64(),
        _ => None,
      };
      if let Some(num) = num {
        let result = match operation {
          RoundingOperation::Absolute => num.abs(),
          RoundingOperation::Round => num.round(),
          RoundingOperation::Ceil => num.ceil(),
          RoundingOperation::Floor => num.floor(),
          RoundingOperation::Trunc => num.trunc(),
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
        .unwrap_or_else(|| "math_rounding_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

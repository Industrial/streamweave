//! Math trigonometric node for performing trigonometric operations.
//!
//! This module provides [`MathTrigonometricNode`], a graph node that performs trigonometric
//! operations on numeric values. It wraps [`MathTrigonometricTransformer`] for use in
//! StreamWeave graphs. It supports operations like Sin, Cos, Tan, Asin, Acos, Atan,
//! and Atan2.
//!
//! # Overview
//!
//! [`MathTrigonometricNode`] is useful for performing trigonometric operations on numeric
//! values in graph-based pipelines. It processes JSON numeric values and applies
//! trigonometric functions, making it ideal for mathematical computations.
//!
//! # Key Concepts
//!
//! - **Trigonometric Functions**: Supports Sin, Cos, Tan and their inverses
//! - **Atan2 Support**: Supports Atan2 with optional second operand
//! - **JSON Value Support**: Works with `serde_json::Value` numeric types
//! - **Transformer Wrapper**: Wraps `MathTrigonometricTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`MathTrigonometricNode`]**: Node that performs trigonometric operations
//! - **[`TrigonometricFunction`]**: Enum representing different trigonometric functions
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::MathTrigonometricNode;
//! use streamweave::transformers::TrigonometricFunction;
//!
//! // Calculate sine
//! let sin = MathTrigonometricNode::new(TrigonometricFunction::Sin, None);
//!
//! // Calculate arctangent2
//! let atan2 = MathTrigonometricNode::new(TrigonometricFunction::Atan2, Some(1.0));
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::MathTrigonometricNode;
//! use streamweave::transformers::TrigonometricFunction;
//! use streamweave::ErrorStrategy;
//!
//! // Create a trigonometric node with error handling
//! let trig = MathTrigonometricNode::new(TrigonometricFunction::Sin, None)
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
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`MathTrigonometricNode`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{MathTrigonometricTransformer, TrigonometricFunction};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that performs trigonometric operations.
///
/// This node wraps `MathTrigonometricTransformer` for use in graphs.
pub struct MathTrigonometricNode {
  /// The underlying trigonometric transformer
  transformer: MathTrigonometricTransformer,
}

impl MathTrigonometricNode {
  /// Creates a new `MathTrigonometricNode`.
  ///
  /// # Arguments
  ///
  /// * `function` - The function to perform.
  /// * `second_operand` - Optional second operand for Atan2.
  pub fn new(function: TrigonometricFunction, second_operand: Option<f64>) -> Self {
    Self {
      transformer: MathTrigonometricTransformer::new(function, second_operand),
    }
  }

  /// Sets the error handling strategy for this node.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Value>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl Clone for MathTrigonometricNode {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for MathTrigonometricNode {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathTrigonometricNode {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathTrigonometricNode {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Value>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<Value> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Value> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<Value>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<Value>) -> ErrorContext<Value> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}

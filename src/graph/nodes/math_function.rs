//! Math function node for performing power and root operations.
//!
//! This module provides [`MathFunctionNode`], a graph node that performs power and
//! root operations on numeric values. It wraps [`MathFunctionTransformer`] for use
//! in StreamWeave graphs. It supports operations like Power, Square, Cube, SquareRoot,
//! CubeRoot, and Absolute.
//!
//! # Overview
//!
//! [`MathFunctionNode`] is useful for performing mathematical power and root operations
//! on numeric values in graph-based pipelines. It processes JSON numeric values and
//! applies power/root functions, making it ideal for mathematical computations.
//!
//! # Key Concepts
//!
//! - **Power Operations**: Supports Power, Square, and Cube operations
//! - **Root Operations**: Supports SquareRoot, CubeRoot, and Nth root operations
//! - **Absolute Value**: Supports absolute value computation
//! - **Transformer Wrapper**: Wraps `MathFunctionTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`MathFunctionNode`]**: Node that performs power and root operations
//! - **[`MathFunction`]**: Enum representing different math functions
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::MathFunctionNode;
//! use streamweave::transformers::MathFunction;
//!
//! // Square a number
//! let square = MathFunctionNode::new(MathFunction::Square, None);
//!
//! // Raise to a power
//! let power = MathFunctionNode::new(MathFunction::Power, Some(3.0));
//!
//! // Square root
//! let sqrt = MathFunctionNode::new(MathFunction::SquareRoot, None);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::MathFunctionNode;
//! use streamweave::transformers::MathFunction;
//! use streamweave::ErrorStrategy;
//!
//! // Create a math function node with error handling
//! let math_func = MathFunctionNode::new(MathFunction::Square, None)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("square".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible numeric
//!   value handling
//! - **Function Enum**: Uses enum-based function selection for type safety
//! - **Optional Operand**: Supports optional operands for functions that need them
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`MathFunctionNode`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{MathFunction, MathFunctionTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that performs power and root operations.
///
/// This node wraps `MathFunctionTransformer` for use in graphs.
pub struct MathFunctionNode {
  /// The underlying function transformer
  transformer: MathFunctionTransformer,
}

impl MathFunctionNode {
  /// Creates a new `MathFunctionNode`.
  ///
  /// # Arguments
  ///
  /// * `function` - The function to perform.
  /// * `exponent` - Optional exponent for Power operation.
  pub fn new(function: MathFunction, exponent: Option<f64>) -> Self {
    Self {
      transformer: MathFunctionTransformer::new(function, exponent),
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

impl Clone for MathFunctionNode {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for MathFunctionNode {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathFunctionNode {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathFunctionNode {
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

//! Math operation node for performing arithmetic operations on numeric values.
//!
//! This module provides [`MathOperationNode`], a graph node that performs arithmetic
//! operations on numeric values. It wraps [`MathOperationTransformer`] for use in
//! StreamWeave graphs. It supports operations like Add, Subtract, Multiply, Divide,
//! Modulo, and Pow.
//!
//! # Overview
//!
//! [`MathOperationNode`] is useful for performing arithmetic operations on numeric
//! values in graph-based pipelines. It processes JSON numeric values and applies
//! arithmetic operations with a specified operand, making it ideal for mathematical
//! computations.
//!
//! # Key Concepts
//!
//! - **Arithmetic Operations**: Supports Add, Subtract, Multiply, Divide, Modulo, Pow
//! - **Binary Operations**: Performs binary operations with a specified operand
//! - **JSON Value Support**: Works with `serde_json::Value` numeric types
//! - **Transformer Wrapper**: Wraps `MathOperationTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`MathOperationNode`]**: Node that performs arithmetic operations
//! - **[`MathOperation`]**: Enum representing different arithmetic operations
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::MathOperationNode;
//! use streamweave::transformers::MathOperation;
//!
//! // Add 10 to each number
//! let add = MathOperationNode::new(MathOperation::Add, 10.0);
//!
//! // Multiply by 2
//! let multiply = MathOperationNode::new(MathOperation::Multiply, 2.0);
//!
//! // Divide by 5
//! let divide = MathOperationNode::new(MathOperation::Divide, 5.0);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::MathOperationNode;
//! use streamweave::transformers::MathOperation;
//! use streamweave::ErrorStrategy;
//!
//! // Create a math operation node with error handling
//! let math_op = MathOperationNode::new(MathOperation::Add, 5.0)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("adder".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible numeric
//!   value handling
//! - **Operation Enum**: Uses enum-based operation selection for type safety
//! - **Operand Required**: Requires an operand value for binary operations
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`MathOperationNode`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{MathOperation, MathOperationTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that performs arithmetic operations.
///
/// This node wraps `MathOperationTransformer` for use in graphs.
pub struct MathOperationNode {
  /// The underlying operation transformer
  transformer: MathOperationTransformer,
}

impl MathOperationNode {
  /// Creates a new `MathOperationNode`.
  ///
  /// # Arguments
  ///
  /// * `operation` - The operation to perform.
  /// * `operand` - The operand value.
  pub fn new(operation: MathOperation, operand: f64) -> Self {
    Self {
      transformer: MathOperationTransformer::new(operation, operand),
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

impl Clone for MathOperationNode {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for MathOperationNode {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathOperationNode {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathOperationNode {
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

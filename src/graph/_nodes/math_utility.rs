//! Math utility node for performing utility math operations.
//!
//! This module provides [`MathUtilityNode`], a graph node that performs utility math
//! operations on numeric values. It wraps [`MathUtilityTransformer`] for use in
//! StreamWeave graphs. It supports utility operations like Hypot, Imul, Sign, Clz32,
//! and Fround.
//!
//! # Overview
//!
//! [`MathUtilityNode`] is useful for performing utility mathematical operations on
//! numeric values in graph-based pipelines. It processes JSON numeric values and
//! applies utility functions, making it ideal for mathematical computations and
//! conversions.
//!
//! # Key Concepts
//!
//! - **Utility Operations**: Supports Hypot, Imul, Sign, Clz32, Fround
//! - **Binary Operations**: Some operations (Hypot, Imul) support optional second operand
//! - **JSON Value Support**: Works with `serde_json::Value` numeric types
//! - **Transformer Wrapper**: Wraps `MathUtilityTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`MathUtilityNode`]**: Node that performs utility math operations
//! - **[`MathUtilityFunction`]**: Enum representing different utility functions
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::MathUtilityNode;
//! use streamweave::transformers::MathUtilityFunction;
//!
//! // Calculate sign
//! let sign = MathUtilityNode::new(MathUtilityFunction::Sign, None);
//!
//! // Calculate hypotenuse
//! let hypot = MathUtilityNode::new(MathUtilityFunction::Hypot, Some(3.0));
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::MathUtilityNode;
//! use streamweave::transformers::MathUtilityFunction;
//! use streamweave::ErrorStrategy;
//!
//! // Create a utility node with error handling
//! let utility = MathUtilityNode::new(MathUtilityFunction::Sign, None)
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
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`MathUtilityNode`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{MathUtilityFunction, MathUtilityTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that performs utility math operations.
///
/// This node wraps `MathUtilityTransformer` for use in graphs.
pub struct MathUtilityNode {
  /// The underlying utility transformer
  transformer: MathUtilityTransformer,
}

impl MathUtilityNode {
  /// Creates a new `MathUtilityNode`.
  ///
  /// # Arguments
  ///
  /// * `function` - The function to perform.
  /// * `second_operand` - Optional second operand for Hypot/Imul.
  pub fn new(function: MathUtilityFunction, second_operand: Option<f64>) -> Self {
    Self {
      transformer: MathUtilityTransformer::new(function, second_operand),
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

impl Clone for MathUtilityNode {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for MathUtilityNode {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathUtilityNode {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathUtilityNode {
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

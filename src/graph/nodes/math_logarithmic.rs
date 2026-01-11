//! Math logarithmic node for performing logarithmic and exponential operations.
//!
//! This module provides [`MathLogarithmicNode`], a graph node that performs logarithmic
//! and exponential operations on numeric values. It wraps [`MathLogarithmicTransformer`]
//! for use in StreamWeave graphs. It supports operations like Log, Log2, Log10, Log1p,
//! Exp, Exp2, and Expm1.
//!
//! # Overview
//!
//! [`MathLogarithmicNode`] is useful for performing logarithmic and exponential operations
//! on numeric values in graph-based pipelines. It processes JSON numeric values and
//! applies logarithmic/exponential functions, making it ideal for mathematical computations.
//!
//! # Key Concepts
//!
//! - **Logarithmic Functions**: Supports Log, Log2, Log10, Log1p
//! - **Exponential Functions**: Supports Exp, Exp2, Expm1
//! - **JSON Value Support**: Works with `serde_json::Value` numeric types
//! - **Transformer Wrapper**: Wraps `MathLogarithmicTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`MathLogarithmicNode`]**: Node that performs logarithmic and exponential operations
//! - **[`LogarithmicFunction`]**: Enum representing different logarithmic/exponential functions
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::MathLogarithmicNode;
//! use streamweave::transformers::LogarithmicFunction;
//!
//! // Calculate natural logarithm
//! let log = MathLogarithmicNode::new(LogarithmicFunction::Log);
//!
//! // Calculate exponential
//! let exp = MathLogarithmicNode::new(LogarithmicFunction::Exp);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::MathLogarithmicNode;
//! use streamweave::transformers::LogarithmicFunction;
//! use streamweave::ErrorStrategy;
//!
//! // Create a logarithmic node with error handling
//! let logarithmic = MathLogarithmicNode::new(LogarithmicFunction::Log)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("log".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible numeric
//!   value handling
//! - **Function Enum**: Uses enum-based function selection for type safety
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`MathLogarithmicNode`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{LogarithmicFunction, MathLogarithmicTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that performs logarithmic and exponential operations.
///
/// This node wraps `MathLogarithmicTransformer` for use in graphs.
pub struct MathLogarithmicNode {
  /// The underlying logarithmic transformer
  transformer: MathLogarithmicTransformer,
}

impl MathLogarithmicNode {
  /// Creates a new `MathLogarithmicNode`.
  ///
  /// # Arguments
  ///
  /// * `function` - The function to perform.
  pub fn new(function: LogarithmicFunction) -> Self {
    Self {
      transformer: MathLogarithmicTransformer::new(function),
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

impl Clone for MathLogarithmicNode {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for MathLogarithmicNode {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathLogarithmicNode {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathLogarithmicNode {
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

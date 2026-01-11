//! Math hyperbolic node for performing hyperbolic operations.
//!
//! This module provides [`MathHyperbolicNode`], a graph node that performs hyperbolic
//! operations on numeric values. It wraps [`MathHyperbolicTransformer`] for use in
//! StreamWeave graphs. It supports operations like Sinh, Cosh, Tanh, Asinh, Acosh,
//! and Atanh.
//!
//! # Overview
//!
//! [`MathHyperbolicNode`] is useful for performing hyperbolic mathematical operations
//! on numeric values in graph-based pipelines. It processes JSON numeric values and
//! applies hyperbolic functions, making it ideal for mathematical computations.
//!
//! # Key Concepts
//!
//! - **Hyperbolic Functions**: Supports Sinh, Cosh, Tanh and their inverses
//! - **JSON Value Support**: Works with `serde_json::Value` numeric types
//! - **Function Enum**: Uses enum-based function selection for type safety
//! - **Transformer Wrapper**: Wraps `MathHyperbolicTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`MathHyperbolicNode`]**: Node that performs hyperbolic operations
//! - **[`HyperbolicFunction`]**: Enum representing different hyperbolic functions
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::MathHyperbolicNode;
//! use streamweave::transformers::HyperbolicFunction;
//!
//! // Calculate hyperbolic sine
//! let sinh = MathHyperbolicNode::new(HyperbolicFunction::Sinh);
//!
//! // Calculate hyperbolic cosine
//! let cosh = MathHyperbolicNode::new(HyperbolicFunction::Cosh);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::MathHyperbolicNode;
//! use streamweave::transformers::HyperbolicFunction;
//! use streamweave::ErrorStrategy;
//!
//! // Create a hyperbolic node with error handling
//! let hyperbolic = MathHyperbolicNode::new(HyperbolicFunction::Sinh)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("sinh".to_string());
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
//! [`MathHyperbolicNode`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{HyperbolicFunction, MathHyperbolicTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that performs hyperbolic operations.
///
/// This node wraps `MathHyperbolicTransformer` for use in graphs.
pub struct MathHyperbolicNode {
  /// The underlying hyperbolic transformer
  transformer: MathHyperbolicTransformer,
}

impl MathHyperbolicNode {
  /// Creates a new `MathHyperbolicNode`.
  ///
  /// # Arguments
  ///
  /// * `function` - The function to perform.
  pub fn new(function: HyperbolicFunction) -> Self {
    Self {
      transformer: MathHyperbolicTransformer::new(function),
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

impl Clone for MathHyperbolicNode {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for MathHyperbolicNode {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathHyperbolicNode {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathHyperbolicNode {
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

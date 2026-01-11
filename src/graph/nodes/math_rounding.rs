//! Math rounding node for performing rounding and absolute value operations.
//!
//! This module provides [`MathRoundingNode`], a graph node that performs rounding
//! and absolute value operations on numeric values. It wraps [`MathRoundingTransformer`]
//! for use in StreamWeave graphs. It supports operations like Round, Floor, Ceil,
//! Trunc, and Abs.
//!
//! # Overview
//!
//! [`MathRoundingNode`] is useful for performing rounding and absolute value operations
//! on numeric values in graph-based pipelines. It processes JSON numeric values and
//! applies rounding/absolute functions, making it ideal for mathematical computations.
//!
//! # Key Concepts
//!
//! - **Rounding Operations**: Supports Round, Floor, Ceil, Trunc
//! - **Absolute Value**: Supports Abs operation
//! - **JSON Value Support**: Works with `serde_json::Value` numeric types
//! - **Transformer Wrapper**: Wraps `MathRoundingTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`MathRoundingNode`]**: Node that performs rounding and absolute value operations
//! - **[`RoundingOperation`]**: Enum representing different rounding operations
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::MathRoundingNode;
//! use streamweave::transformers::RoundingOperation;
//!
//! // Round to nearest integer
//! let round = MathRoundingNode::new(RoundingOperation::Round);
//!
//! // Floor (round down)
//! let floor = MathRoundingNode::new(RoundingOperation::Floor);
//!
//! // Absolute value
//! let abs = MathRoundingNode::new(RoundingOperation::Abs);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::MathRoundingNode;
//! use streamweave::transformers::RoundingOperation;
//! use streamweave::ErrorStrategy;
//!
//! // Create a rounding node with error handling
//! let rounding = MathRoundingNode::new(RoundingOperation::Round)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("rounder".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible numeric
//!   value handling
//! - **Operation Enum**: Uses enum-based operation selection for type safety
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`MathRoundingNode`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{MathRoundingTransformer, RoundingOperation};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that performs rounding and absolute value operations.
///
/// This node wraps `MathRoundingTransformer` for use in graphs.
pub struct MathRoundingNode {
  /// The underlying rounding transformer
  transformer: MathRoundingTransformer,
}

impl MathRoundingNode {
  /// Creates a new `MathRoundingNode`.
  ///
  /// # Arguments
  ///
  /// * `operation` - The operation to perform.
  pub fn new(operation: RoundingOperation) -> Self {
    Self {
      transformer: MathRoundingTransformer::new(operation),
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

impl Clone for MathRoundingNode {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for MathRoundingNode {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathRoundingNode {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathRoundingNode {
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

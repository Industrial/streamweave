//! Math min/max node for finding minimum or maximum values.
//!
//! This module provides [`MathMinMaxNode`], a graph node that finds minimum or maximum
//! values. It wraps [`MathMinMaxTransformer`] for use in StreamWeave graphs. It supports
//! both Min and Max operations, optionally comparing against a reference value.
//!
//! # Overview
//!
//! [`MathMinMaxNode`] is useful for finding minimum or maximum values in graph-based
//! pipelines. It processes JSON numeric values and finds the minimum or maximum,
//! optionally comparing against a reference value, making it ideal for mathematical
//! computations and filtering.
//!
//! # Key Concepts
//!
//! - **Min/Max Operations**: Supports finding minimum or maximum values
//! - **Reference Comparison**: Optionally compares against a reference value
//! - **JSON Value Support**: Works with `serde_json::Value` numeric types
//! - **Transformer Wrapper**: Wraps `MathMinMaxTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`MathMinMaxNode`]**: Node that finds minimum or maximum values
//! - **[`MinMaxOperation`]**: Enum representing Min or Max operations
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::MathMinMaxNode;
//! use streamweave::transformers::MinMaxOperation;
//!
//! // Find maximum value
//! let max = MathMinMaxNode::new(MinMaxOperation::Max, None);
//!
//! // Find minimum value compared to 10
//! let min = MathMinMaxNode::new(MinMaxOperation::Min, Some(10.0));
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::MathMinMaxNode;
//! use streamweave::transformers::MinMaxOperation;
//! use streamweave::ErrorStrategy;
//!
//! // Create a min/max node with error handling
//! let min_max = MathMinMaxNode::new(MinMaxOperation::Max, None)
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
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`MathMinMaxNode`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{MathMinMaxTransformer, MinMaxOperation};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that finds minimum or maximum values.
///
/// This node wraps `MathMinMaxTransformer` for use in graphs.
pub struct MathMinMaxNode {
  /// The underlying min/max transformer
  transformer: MathMinMaxTransformer,
}

impl MathMinMaxNode {
  /// Creates a new `MathMinMaxNode`.
  ///
  /// # Arguments
  ///
  /// * `operation` - The operation to perform (Min or Max).
  /// * `reference` - Optional reference value for single number comparison.
  pub fn new(operation: MinMaxOperation, reference: Option<f64>) -> Self {
    Self {
      transformer: MathMinMaxTransformer::new(operation, reference),
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

impl Clone for MathMinMaxNode {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for MathMinMaxNode {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathMinMaxNode {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathMinMaxNode {
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

//! Array concatenation node for StreamWeave graphs.
//!
//! This module provides [`ArrayConcat`], a graph node that concatenates multiple
//! arrays into a single array. It wraps [`ArrayConcatTransformer`] for use in
//! StreamWeave graph topologies.
//!
//! # Overview
//!
//! [`ArrayConcat`] is useful for combining arrays from multiple sources or flattening
//! nested array structures in graph-based processing pipelines. It processes JSON
//! arrays and concatenates their elements into a single array.
//!
//! # Key Concepts
//!
//! - **Array Concatenation**: Combines multiple arrays into one
//! - **JSON Processing**: Works with `serde_json::Value` arrays
//! - **Nested Arrays**: Handles arrays of arrays
//! - **Graph Integration**: Wraps transformer for use in graph topologies
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`ArrayConcat`]**: Graph node that concatenates arrays
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::ArrayConcat;
//! use serde_json::json;
//!
//! // Create a node that concatenates arrays
//! let node = ArrayConcat::new()
//!     .with_name("concat-arrays".to_string());
//!
//! // Input: json!([[1, 2], [3, 4]])
//! // Output: json!([1, 2, 3, 4])
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::ArrayConcat;
//! use streamweave::ErrorStrategy;
//! use serde_json::Value;
//!
//! // Create a node with error handling strategy
//! let node = ArrayConcat::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("concat-node".to_string());
//! ```
//!
//! # Behavior
//!
//! The node takes an array of arrays as input and concatenates all sub-arrays into
//! a single array. Non-array values are passed through unchanged.
//!
//! # Design Decisions
//!
//! - **Transformer Wrapper**: Wraps `ArrayConcatTransformer` to provide graph node interface
//! - **JSON Value Type**: Uses `serde_json::Value` for flexible JSON array handling
//! - **Simple Concatenation**: Directly concatenates array elements
//! - **Type Safety**: Validates input is an array before processing
//! - **Graph Integration**: Seamlessly integrates with StreamWeave graph execution
//!
//! # Integration with StreamWeave
//!
//! [`ArrayConcat`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ArrayConcatTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that concatenates arrays.
///
/// This node wraps `ArrayConcatTransformer` for use in graphs.
pub struct ArrayConcat {
  /// The underlying concat transformer
  transformer: ArrayConcatTransformer,
}

impl ArrayConcat {
  /// Creates a new `ArrayConcat` node.
  pub fn new() -> Self {
    Self {
      transformer: ArrayConcatTransformer::new(),
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

impl Default for ArrayConcat {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ArrayConcat {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for ArrayConcat {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ArrayConcat {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ArrayConcat {
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

// Tests for ArrayConcat node

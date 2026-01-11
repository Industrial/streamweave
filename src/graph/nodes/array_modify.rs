//! Array modify node for modifying JSON arrays.
//!
//! This module provides [`ArrayModify`], a graph node that modifies JSON arrays
//! using operations like Push, Pop, Shift, and Unshift. It wraps
//! [`ArrayModifyTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`ArrayModify`] is useful for manipulating JSON array values in graph-based
//! pipelines. It supports common array modification operations that add or remove
//! elements from arrays, making it ideal for data transformation workflows.
//!
//! # Key Concepts
//!
//! - **Array Operations**: Supports Push, Pop, Shift, and Unshift operations
//! - **JSON Values**: Works with `serde_json::Value` array types
//! - **In-Place Modification**: Modifies arrays in place (consumes and produces
//!   modified arrays)
//! - **Transformer Wrapper**: Wraps `ArrayModifyTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`ArrayModify`]**: Node that modifies arrays using specified operations
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::ArrayModify;
//! use streamweave::transformers::ArrayModifyOperation;
//! use serde_json::json;
//!
//! // Create a node that pushes a value to arrays
//! let node = ArrayModify::new(
//!     ArrayModifyOperation::Push,
//!     Some(json!("new_value"))
//! );
//! ```
//!
//! ## Different Operations
//!
//! ```rust
//! use streamweave::graph::nodes::ArrayModify;
//! use streamweave::transformers::ArrayModifyOperation;
//!
//! // Push: Add element to end
//! let push_node = ArrayModify::new(ArrayModifyOperation::Push, Some(json!(42)));
//!
//! // Pop: Remove element from end
//! let pop_node = ArrayModify::new(ArrayModifyOperation::Pop, None);
//!
//! // Shift: Remove element from beginning
//! let shift_node = ArrayModify::new(ArrayModifyOperation::Shift, None);
//!
//! // Unshift: Add element to beginning
//! let unshift_node = ArrayModify::new(ArrayModifyOperation::Unshift, Some(json!(42)));
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Uses `serde_json::Value` for flexible array handling
//! - **Operation Enum**: Uses `ArrayModifyOperation` enum for type-safe operation
//!   specification
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`ArrayModify`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{ArrayModifyOperation, ArrayModifyTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that modifies arrays.
///
/// This node wraps `ArrayModifyTransformer` for use in graphs.
pub struct ArrayModify {
  /// The underlying modify transformer
  transformer: ArrayModifyTransformer,
}

impl ArrayModify {
  /// Creates a new `ArrayModify` node.
  ///
  /// # Arguments
  ///
  /// * `operation` - The operation to perform (Push, Pop, Shift, or Unshift).
  /// * `value` - Optional value to add (required for Push/Unshift, ignored for Pop/Shift).
  pub fn new(operation: ArrayModifyOperation, value: Option<Value>) -> Self {
    Self {
      transformer: ArrayModifyTransformer::new(operation, value),
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

impl Clone for ArrayModify {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for ArrayModify {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ArrayModify {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ArrayModify {
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

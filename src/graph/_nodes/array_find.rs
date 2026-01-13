//! Array find node for finding elements or indices in JSON arrays.
//!
//! This module provides [`ArrayFind`], a graph node that finds elements or indices
//! in JSON arrays. It wraps [`ArrayFindTransformer`] for use in StreamWeave graphs.
//! It supports both Find (returns matching elements) and FindIndex (returns indices
//! of matching elements) operations.
//!
//! # Overview
//!
//! [`ArrayFind`] is useful for finding elements or indices in JSON arrays in
//! graph-based pipelines. It processes JSON arrays and finds elements or indices
//! that match a search value, making it ideal for array searching and filtering.
//!
//! # Key Concepts
//!
//! - **Array Searching**: Finds elements or indices in JSON arrays
//! - **Find Operation**: Returns matching elements from arrays
//! - **FindIndex Operation**: Returns indices of matching elements
//! - **Transformer Wrapper**: Wraps `ArrayFindTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`ArrayFind`]**: Node that finds elements or indices in arrays
//! - **[`FindOperation`]**: Enum representing Find or FindIndex operations
//!
//! # Quick Start
//!
//! ## Basic Usage (Find Elements)
//!
//! ```rust
//! use streamweave::graph::nodes::ArrayFind;
//! use streamweave::transformers::FindOperation;
//! use serde_json::json;
//!
//! // Find elements matching a value
//! let array_find = ArrayFind::new(
//!     FindOperation::Find,
//!     json!("target")
//! );
//! ```
//!
//! ## Find Indices
//!
//! ```rust
//! use streamweave::graph::nodes::ArrayFind;
//! use streamweave::transformers::FindOperation;
//! use serde_json::json;
//!
//! // Find indices of matching elements
//! let array_find = ArrayFind::new(
//!     FindOperation::FindIndex,
//!     json!(42)
//! );
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::ArrayFind;
//! use streamweave::transformers::FindOperation;
//! use streamweave::ErrorStrategy;
//! use serde_json::json;
//!
//! // Create an array find node with error handling
//! let array_find = ArrayFind::new(FindOperation::Find, json!("target"))
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("array-finder".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible JSON
//!   array handling
//! - **Operation Enum**: Uses enum-based operation selection for type safety
//! - **Search Value**: Supports flexible search value specification
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`ArrayFind`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{ArrayFindTransformer, FindOperation};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
#[allow(unused_imports)]
use futures::StreamExt;
#[allow(unused_imports)]
use futures::stream;
use serde_json::Value;
#[allow(unused_imports)]
use serde_json::json;
use std::pin::Pin;

/// Node that finds elements or indices in arrays.
///
/// This node wraps `ArrayFindTransformer` for use in graphs.
pub struct ArrayFind {
  /// The underlying find transformer
  transformer: ArrayFindTransformer,
}

impl ArrayFind {
  /// Creates a new `ArrayFind` node.
  ///
  /// # Arguments
  ///
  /// * `operation` - The operation to perform (Find or FindIndex).
  /// * `search_value` - The value to find.
  pub fn new(operation: FindOperation, search_value: Value) -> Self {
    Self {
      transformer: ArrayFindTransformer::new(operation, search_value),
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

impl Clone for ArrayFind {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for ArrayFind {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ArrayFind {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ArrayFind {
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

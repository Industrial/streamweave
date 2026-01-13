//! Array slice node for extracting slices from JSON arrays.
//!
//! This module provides [`ArraySlice`], a graph node that extracts slices from
//! JSON arrays using start and end indices. It wraps [`ArraySliceTransformer`]
//! for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`ArraySlice`] is useful for extracting sub-arrays from JSON arrays in
//! graph-based pipelines. It supports both bounded slices (with end index) and
//! unbounded slices (to end of array), similar to Rust's slice operations.
//!
//! # Key Concepts
//!
//! - **Array Slicing**: Extracts contiguous sub-arrays from arrays
//! - **Index-Based**: Uses start and optional end indices for slice extraction
//! - **JSON Values**: Works with `serde_json::Value` array types
//! - **Transformer Wrapper**: Wraps `ArraySliceTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`ArraySlice`]**: Node that extracts slices from arrays
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::ArraySlice;
//!
//! // Create a node that extracts slices from index 2 to 5
//! let node = ArraySlice::new(2, Some(5));
//!
//! // Create a node that extracts from index 2 to end of array
//! let node_unbounded = ArraySlice::new(2, None);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::ArraySlice;
//! use streamweave::ErrorStrategy;
//!
//! // Create a node with error handling
//! let node = ArraySlice::new(0, Some(10))
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("array-slicer".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Uses `serde_json::Value` for flexible array handling
//! - **Rust-Style Slicing**: Follows Rust's slice semantics (start inclusive,
//!   end exclusive)
//! - **Optional End Index**: Supports unbounded slices with `None` end index
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`ArraySlice`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ArraySliceTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that extracts slices from arrays.
///
/// This node wraps `ArraySliceTransformer` for use in graphs.
pub struct ArraySlice {
  /// The underlying slice transformer
  transformer: ArraySliceTransformer,
}

impl ArraySlice {
  /// Creates a new `ArraySlice` node.
  ///
  /// # Arguments
  ///
  /// * `start` - Start index (inclusive).
  /// * `end` - End index (exclusive), or None to extract to end of array.
  pub fn new(start: usize, end: Option<usize>) -> Self {
    Self {
      transformer: ArraySliceTransformer::new(start, end),
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

impl Clone for ArraySlice {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for ArraySlice {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ArraySlice {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ArraySlice {
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

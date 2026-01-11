//! Object merge node for merging multiple JSON objects into a single object.
//!
//! This module provides [`ObjectMerge`], a graph node that merges multiple JSON
//! objects into a single object. It wraps [`ObjectMergeTransformer`] for use in
//! StreamWeave graphs. It combines properties from multiple objects, with later
//! objects overriding earlier ones for duplicate keys.
//!
//! # Overview
//!
//! [`ObjectMerge`] is useful for merging multiple JSON objects in graph-based
//! pipelines. It processes multiple objects and outputs a single merged object,
//! making it ideal for combining data from different sources or enriching objects
//! with additional properties.
//!
//! # Key Concepts
//!
//! - **Object Merging**: Merges multiple JSON objects into a single object
//! - **Property Override**: Later objects override earlier ones for duplicate keys
//! - **JSON Value Support**: Works with `serde_json::Value` objects
//! - **Transformer Wrapper**: Wraps `ObjectMergeTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`ObjectMerge`]**: Node that merges multiple JSON objects
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::ObjectMerge;
//!
//! // Create an object merge node
//! let object_merge = ObjectMerge::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::ObjectMerge;
//! use streamweave::ErrorStrategy;
//!
//! // Create an object merge node with error handling
//! let object_merge = ObjectMerge::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("object-merger".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible JSON
//!   object handling
//! - **Property Override**: Later objects override earlier ones for predictable
//!   merge behavior
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`ObjectMerge`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ObjectMergeTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that merges JSON objects.
///
/// This node wraps `ObjectMergeTransformer` for use in graphs.
pub struct ObjectMerge {
  /// The underlying merge transformer
  transformer: ObjectMergeTransformer,
}

impl ObjectMerge {
  /// Creates a new `ObjectMerge` node.
  pub fn new() -> Self {
    Self {
      transformer: ObjectMergeTransformer::new(),
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

impl Default for ObjectMerge {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ObjectMerge {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for ObjectMerge {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ObjectMerge {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ObjectMerge {
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

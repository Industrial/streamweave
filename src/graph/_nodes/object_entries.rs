//! Object entries node for extracting key-value pairs from JSON objects.
//!
//! This module provides [`ObjectEntries`], a graph node that extracts key-value
//! pairs from JSON objects. It wraps [`ObjectEntriesTransformer`] for use in
//! StreamWeave graphs. It converts JSON objects into arrays of key-value pairs,
//! making it ideal for object iteration and processing.
//!
//! # Overview
//!
//! [`ObjectEntries`] is useful for extracting key-value pairs from JSON objects
//! in graph-based pipelines. It processes JSON objects and outputs arrays of
//! [key, value] pairs, making it ideal for object iteration, transformation,
//! and processing.
//!
//! # Key Concepts
//!
//! - **Key-Value Extraction**: Extracts key-value pairs from JSON objects
//! - **Array Output**: Outputs arrays of [key, value] pairs from objects
//! - **JSON Value Support**: Works with `serde_json::Value` objects
//! - **Transformer Wrapper**: Wraps `ObjectEntriesTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`ObjectEntries`]**: Node that extracts key-value pairs from JSON objects
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::ObjectEntries;
//!
//! // Create an object entries node
//! let object_entries = ObjectEntries::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::ObjectEntries;
//! use streamweave::ErrorStrategy;
//!
//! // Create an object entries node with error handling
//! let object_entries = ObjectEntries::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("entries-extractor".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible JSON
//!   object handling
//! - **Array Output**: Outputs arrays of [key, value] pairs for easy iteration
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`ObjectEntries`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ObjectEntriesTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that extracts key-value pairs from JSON objects.
///
/// This node wraps `ObjectEntriesTransformer` for use in graphs.
pub struct ObjectEntries {
  /// The underlying entries transformer
  transformer: ObjectEntriesTransformer,
}

impl ObjectEntries {
  /// Creates a new `ObjectEntries` node.
  pub fn new() -> Self {
    Self {
      transformer: ObjectEntriesTransformer::new(),
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

impl Default for ObjectEntries {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ObjectEntries {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for ObjectEntries {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ObjectEntries {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ObjectEntries {
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

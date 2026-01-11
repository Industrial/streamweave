//! Object keys node for extracting keys from JSON objects.
//!
//! This module provides [`ObjectKeys`], a graph node that extracts keys from JSON
//! objects. It wraps [`ObjectKeysTransformer`] for use in StreamWeave graphs. It
//! converts JSON objects into arrays of their keys, making it ideal for object
//! introspection and processing.
//!
//! # Overview
//!
//! [`ObjectKeys`] is useful for extracting keys from JSON objects in graph-based
//! pipelines. It processes JSON objects and outputs arrays of their keys, making
//! it ideal for object introspection, validation, and processing.
//!
//! # Key Concepts
//!
//! - **Key Extraction**: Extracts keys from JSON objects
//! - **Array Output**: Outputs arrays of keys from objects
//! - **JSON Value Support**: Works with `serde_json::Value` objects
//! - **Transformer Wrapper**: Wraps `ObjectKeysTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`ObjectKeys`]**: Node that extracts keys from JSON objects
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::ObjectKeys;
//!
//! // Create an object keys node
//! let object_keys = ObjectKeys::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::ObjectKeys;
//! use streamweave::ErrorStrategy;
//!
//! // Create an object keys node with error handling
//! let object_keys = ObjectKeys::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("keys-extractor".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible JSON
//!   object handling
//! - **Array Output**: Outputs arrays of keys for easy iteration
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`ObjectKeys`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ObjectKeysTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that extracts keys from JSON objects.
///
/// This node wraps `ObjectKeysTransformer` for use in graphs.
pub struct ObjectKeys {
  /// The underlying keys transformer
  transformer: ObjectKeysTransformer,
}

impl ObjectKeys {
  /// Creates a new `ObjectKeys` node.
  pub fn new() -> Self {
    Self {
      transformer: ObjectKeysTransformer::new(),
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

impl Default for ObjectKeys {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ObjectKeys {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for ObjectKeys {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ObjectKeys {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ObjectKeys {
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

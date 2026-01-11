//! Object has property node for filtering objects that have a specific property.
//!
//! This module provides [`ObjectHasProperty`], a graph node that filters JSON
//! objects that have a specific property. It wraps [`ObjectHasPropertyTransformer`]
//! for use in StreamWeave graphs. It filters out objects that don't have the
//! specified property, making it ideal for data validation and filtering.
//!
//! # Overview
//!
//! [`ObjectHasProperty`] is useful for filtering JSON objects that have a specific
//! property in graph-based pipelines. It processes JSON objects and outputs only
//! those that contain the specified property, making it ideal for data validation
//! and conditional processing.
//!
//! # Key Concepts
//!
//! - **Property Checking**: Checks if objects have a specific property
//! - **Filtering**: Filters out objects that don't have the property
//! - **JSON Value Support**: Works with `serde_json::Value` objects
//! - **Transformer Wrapper**: Wraps `ObjectHasPropertyTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`ObjectHasProperty`]**: Node that filters objects with a specific property
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::ObjectHasProperty;
//!
//! // Filter objects that have the "name" property
//! let has_property = ObjectHasProperty::new("name");
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::ObjectHasProperty;
//! use streamweave::ErrorStrategy;
//!
//! // Create an object has property node with error handling
//! let has_property = ObjectHasProperty::new("email")
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("email-filter".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible JSON
//!   object handling
//! - **Property-Based Filtering**: Filters based on property existence
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`ObjectHasProperty`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ObjectHasPropertyTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that filters objects that have a specific property.
///
/// This node wraps `ObjectHasPropertyTransformer` for use in graphs.
pub struct ObjectHasProperty {
  /// The underlying has property transformer
  transformer: ObjectHasPropertyTransformer,
}

impl ObjectHasProperty {
  /// Creates a new `ObjectHasProperty` node.
  pub fn new(key: impl Into<String>) -> Self {
    Self {
      transformer: ObjectHasPropertyTransformer::new(key),
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

impl Clone for ObjectHasProperty {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for ObjectHasProperty {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ObjectHasProperty {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ObjectHasProperty {
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

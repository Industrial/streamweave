//! Object property node for getting, setting, or deleting properties from JSON objects.
//!
//! This module provides [`ObjectProperty`], a graph node that performs property
//! operations on JSON objects. It wraps [`ObjectPropertyTransformer`] for use in
//! StreamWeave graphs. It supports getting, setting, and deleting properties,
//! making it ideal for object manipulation and transformation.
//!
//! # Overview
//!
//! [`ObjectProperty`] is useful for performing property operations on JSON objects
//! in graph-based pipelines. It processes JSON objects and performs property
//! operations (get, set, delete), making it ideal for object manipulation,
//! enrichment, and transformation.
//!
//! # Key Concepts
//!
//! - **Property Operations**: Supports Get, Set, and Delete operations
//! - **Property Manipulation**: Gets, sets, or deletes properties from objects
//! - **JSON Value Support**: Works with `serde_json::Value` objects
//! - **Transformer Wrapper**: Wraps `ObjectPropertyTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`ObjectProperty`]**: Node that performs property operations on JSON objects
//! - **[`PropertyOperation`]**: Enum representing different property operations
//!
//! # Quick Start
//!
//! ## Basic Usage (Get)
//!
//! ```rust
//! use streamweave::graph::nodes::ObjectProperty;
//! use streamweave::transformers::PropertyOperation;
//!
//! // Get a property value
//! let get_property = ObjectProperty::new("name", PropertyOperation::Get, None);
//! ```
//!
//! ## Set Property
//!
//! ```rust
//! use streamweave::graph::nodes::ObjectProperty;
//! use streamweave::transformers::PropertyOperation;
//! use serde_json::json;
//!
//! // Set a property value
//! let set_property = ObjectProperty::new(
//!     "status",
//!     PropertyOperation::Set,
//!     Some(json!("active"))
//! );
//! ```
//!
//! ## Delete Property
//!
//! ```rust
//! use streamweave::graph::nodes::ObjectProperty;
//! use streamweave::transformers::PropertyOperation;
//!
//! // Delete a property
//! let delete_property = ObjectProperty::new("old_field", PropertyOperation::Delete, None);
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible JSON
//!   object handling
//! - **Operation Enum**: Uses enum-based operation selection for type safety
//! - **Optional Set Value**: Supports optional set value for Set operation
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`ObjectProperty`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{ObjectPropertyTransformer, PropertyOperation};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that performs property operations on JSON objects.
///
/// This node wraps `ObjectPropertyTransformer` for use in graphs.
pub struct ObjectProperty {
  /// The underlying property transformer
  transformer: ObjectPropertyTransformer,
}

impl ObjectProperty {
  /// Creates a new `ObjectProperty` node.
  ///
  /// # Arguments
  ///
  /// * `key` - The property key to operate on.
  /// * `operation` - The operation to perform (Get, Set, or Delete).
  /// * `set_value` - Optional value to set (required for Set operation).
  pub fn new(
    key: impl Into<String>,
    operation: PropertyOperation,
    set_value: Option<Value>,
  ) -> Self {
    Self {
      transformer: ObjectPropertyTransformer::new(key, operation, set_value),
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

impl Clone for ObjectProperty {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for ObjectProperty {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ObjectProperty {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ObjectProperty {
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

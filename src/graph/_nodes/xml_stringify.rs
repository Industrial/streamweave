//! XML stringification node for StreamWeave graphs.
//!
//! This module provides [`XmlStringify`], a graph node that converts JSON values
//! into XML-formatted strings. It wraps [`XmlStringifyTransformer`] for use in
//! StreamWeave graph topologies.
//!
//! # Overview
//!
//! [`XmlStringify`] is useful for converting structured JSON data into XML format
//! in graph-based processing pipelines. It enables JSON data to be serialized
//! as XML for systems that require XML input.
//!
//! # Key Concepts
//!
//! - **XML Serialization**: Converts JSON `Value` structures to XML strings
//! - **Type Conversion**: Transforms `serde_json::Value` input to `String` output
//! - **Pretty Printing**: Optional pretty-printing for human-readable XML
//! - **Root Element**: Configurable root element name for XML output
//! - **Graph Integration**: Wraps transformer for use in graph topologies
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`XmlStringify`]**: Graph node that converts JSON values to XML strings
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::XmlStringify;
//! use serde_json::json;
//!
//! // Create a node that converts JSON to XML
//! let node = XmlStringify::new()
//!     .with_name("stringify-xml".to_string());
//!
//! // Input: json!({"root": {"item": "value"}})
//! // Output: "<root><item>value</item></root>"
//! ```
//!
//! ## With Pretty Printing
//!
//! ```rust
//! use streamweave::graph::nodes::XmlStringify;
//!
//! // Create a node with pretty-printed XML output
//! let node = XmlStringify::new()
//!     .with_pretty(true)
//!     .with_root_element("document")
//!     .with_name("xml-formatter".to_string());
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::XmlStringify;
//! use streamweave::ErrorStrategy;
//! use serde_json::Value;
//!
//! // Create a node with error handling strategy
//! let node = XmlStringify::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("xml-serializer".to_string());
//! ```
//!
//! # Configuration Options
//!
//! - **Pretty Printing**: Enable formatted XML output with indentation
//! - **Root Element**: Specify the root element name for the XML document
//! - **Error Strategy**: Configure how to handle serialization errors
//!
//! # Design Decisions
//!
//! - **Transformer Wrapper**: Wraps `XmlStringifyTransformer` to provide graph node interface
//! - **JSON Input**: Uses `serde_json::Value` for flexible JSON structure handling
//! - **String Output**: Produces XML as strings for compatibility with text-based systems
//! - **Pretty Printing**: Optional formatting for human-readable output
//! - **Graph Integration**: Seamlessly integrates with StreamWeave graph execution
//!
//! # Integration with StreamWeave
//!
//! [`XmlStringify`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::XmlStringifyTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that stringifies JSON values to XML from stream items.
///
/// This node wraps `XmlStringifyTransformer` for use in graphs.
pub struct XmlStringify {
  /// The underlying stringify transformer
  transformer: XmlStringifyTransformer,
}

impl XmlStringify {
  /// Creates a new `XmlStringify` node.
  pub fn new() -> Self {
    Self {
      transformer: XmlStringifyTransformer::new(),
    }
  }

  /// Sets whether to pretty-print XML.
  pub fn with_pretty(mut self, pretty: bool) -> Self {
    self.transformer = self.transformer.with_pretty(pretty);
    self
  }

  /// Sets the root element name.
  pub fn with_root_element(mut self, root: impl Into<String>) -> Self {
    self.transformer = self.transformer.with_root_element(root);
    self
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

impl Default for XmlStringify {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for XmlStringify {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for XmlStringify {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for XmlStringify {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for XmlStringify {
  type InputPorts = (Value,);
  type OutputPorts = (String,);

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

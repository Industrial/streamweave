//! XML parsing node for StreamWeave graphs.
//!
//! This module provides [`XmlParse`], a graph node that parses XML strings into
//! JSON values. It wraps [`XmlParseTransformer`] for use in StreamWeave graph
//! topologies.
//!
//! # Overview
//!
//! [`XmlParse`] is useful for converting XML-formatted strings into structured
//! JSON values in graph-based processing pipelines. It enables XML data to be
//! processed using StreamWeave's JSON-based transformation capabilities.
//!
//! # Key Concepts
//!
//! - **XML Parsing**: Converts XML strings to JSON `Value` structures
//! - **Type Conversion**: Transforms `String` input to `serde_json::Value` output
//! - **Graph Integration**: Wraps transformer for use in graph topologies
//! - **Error Handling**: Configurable error strategies for malformed XML
//!
//! # Core Types
//!
//! - **[`XmlParse`]**: Graph node that parses XML strings to JSON values
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::XmlParse;
//!
//! // Create a node that parses XML strings
//! let node = XmlParse::new()
//!     .with_name("parse-xml".to_string());
//!
//! // Input: "<root><item>value</item></root>"
//! // Output: json!({"root": {"item": "value"}})
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::XmlParse;
//! use streamweave::ErrorStrategy;
//! use serde_json::Value;
//!
//! // Create a node with error handling strategy
//! let node = XmlParse::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("xml-parser".to_string());
//! ```
//!
//! # Behavior
//!
//! The node takes XML-formatted strings as input and converts them to JSON
//! values. Malformed XML will trigger error handling based on the configured
//! error strategy.
//!
//! # Design Decisions
//!
//! - **Transformer Wrapper**: Wraps `XmlParseTransformer` to provide graph node interface
//! - **JSON Output**: Uses `serde_json::Value` for flexible JSON structure handling
//! - **String Input**: Accepts XML as strings for compatibility with text-based streams
//! - **Error Handling**: Supports configurable error strategies for malformed XML
//! - **Graph Integration**: Seamlessly integrates with StreamWeave graph execution
//!
//! # Integration with StreamWeave
//!
//! [`XmlParse`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::XmlParseTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that parses XML strings from stream items.
///
/// This node wraps `XmlParseTransformer` for use in graphs.
pub struct XmlParse {
  /// The underlying parse transformer
  transformer: XmlParseTransformer,
}

impl XmlParse {
  /// Creates a new `XmlParse` node.
  pub fn new() -> Self {
    Self {
      transformer: XmlParseTransformer::new(),
    }
  }

  /// Sets the error handling strategy for this node.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl Default for XmlParse {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for XmlParse {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for XmlParse {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for XmlParse {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for XmlParse {
  type InputPorts = (String,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}

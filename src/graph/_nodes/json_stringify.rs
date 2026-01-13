//! JSON stringify node for converting JSON values to strings.
//!
//! This module provides [`JsonStringify`], a graph node that converts JSON values
//! to JSON strings. It wraps [`JsonStringifyTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`JsonStringify`] is useful for converting structured JSON values to JSON strings
//! in graph-based pipelines. It serializes JSON values to strings, making it ideal
//! for exporting JSON data as text.
//!
//! # Key Concepts
//!
//! - **JSON Serialization**: Converts JSON values to JSON strings
//! - **JSON Value Input**: Works with `serde_json::Value` types
//! - **String Output**: Produces JSON-formatted strings
//! - **Transformer Wrapper**: Wraps `JsonStringifyTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`JsonStringify`]**: Node that converts JSON values to JSON strings
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::JsonStringify;
//!
//! // Create a JSON stringify node
//! let json_stringify = JsonStringify::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::JsonStringify;
//! use streamweave::ErrorStrategy;
//!
//! // Create a JSON stringify node with error handling
//! let json_stringify = JsonStringify::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("json-stringifier".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Library Integration**: Uses serde_json for robust JSON serialization
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible data
//!   structure handling
//! - **Pretty Printing**: Can optionally format JSON for readability
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`JsonStringify`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::JsonStringifyTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that stringifies JSON values from stream items.
///
/// This node wraps `JsonStringifyTransformer` for use in graphs.
pub struct JsonStringify {
  /// The underlying stringify transformer
  transformer: JsonStringifyTransformer,
}

impl JsonStringify {
  /// Creates a new `JsonStringify` node.
  pub fn new() -> Self {
    Self {
      transformer: JsonStringifyTransformer::new(),
    }
  }

  /// Sets whether to pretty-print JSON.
  pub fn with_pretty(mut self, pretty: bool) -> Self {
    self.transformer = self.transformer.with_pretty(pretty);
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

impl Default for JsonStringify {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for JsonStringify {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for JsonStringify {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for JsonStringify {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for JsonStringify {
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

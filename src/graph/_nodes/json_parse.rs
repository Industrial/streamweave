//! JSON parse node for parsing JSON strings in graphs.
//!
//! This module provides [`JsonParse`], a graph node that parses JSON strings from
//! stream items and converts them to JSON values. It wraps [`JsonParseTransformer`]
//! for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`JsonParse`] is useful for parsing JSON data in graph-based pipelines. It
//! converts JSON strings to structured JSON values, making it ideal for processing
//! JSON data from various sources.
//!
//! # Key Concepts
//!
//! - **JSON Parsing**: Parses JSON strings into structured JSON values
//! - **JSON Value Output**: Produces `serde_json::Value` from JSON strings
//! - **Error Handling**: Handles invalid JSON gracefully according to error strategy
//! - **Transformer Wrapper**: Wraps `JsonParseTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`JsonParse`]**: Node that parses JSON strings from stream items
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::JsonParse;
//!
//! // Create a JSON parse node
//! let json_parse = JsonParse::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::JsonParse;
//! use streamweave::ErrorStrategy;
//!
//! // Create a JSON parse node with error handling
//! let json_parse = JsonParse::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("json-parser".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Library Integration**: Uses serde_json for robust JSON parsing
//! - **JSON Value Output**: Produces JSON values for flexible data structure
//!   handling
//! - **String Input**: Takes JSON strings as input for compatibility with text
//!   sources
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`JsonParse`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::JsonParseTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that parses JSON strings from stream items.
///
/// This node wraps `JsonParseTransformer` for use in graphs.
pub struct JsonParse {
  /// The underlying parse transformer
  transformer: JsonParseTransformer,
}

impl JsonParse {
  /// Creates a new `JsonParse` node.
  pub fn new() -> Self {
    Self {
      transformer: JsonParseTransformer::new(),
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

impl Default for JsonParse {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for JsonParse {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for JsonParse {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for JsonParse {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for JsonParse {
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

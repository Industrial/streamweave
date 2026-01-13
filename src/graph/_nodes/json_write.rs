//! JSON write node for writing data to JSON files while passing data through.
//!
//! This module provides [`JsonWrite`], a graph node that writes data to JSON files
//! while passing the same data through to the output. It takes data as input, writes
//! it to a JSON file, and outputs the same data, enabling writing intermediate results
//! while continuing processing. It wraps [`JsonWriteTransformer`] for use in
//! StreamWeave graphs.
//!
//! # Overview
//!
//! [`JsonWrite`] is useful for writing intermediate results to JSON files while
//! continuing processing in graph-based pipelines. Unlike consumers, it passes data
//! through, making it ideal for checkpointing data at intermediate stages or logging
//! JSON writes.
//!
//! # Key Concepts
//!
//! - **Pass-Through Operation**: Writes data to JSON files while passing it through to output
//! - **JSON Serialization**: Serializes data to JSON format before writing
//! - **Intermediate Results**: Enables writing intermediate results without
//!   interrupting the pipeline
//! - **Transformer Wrapper**: Wraps `JsonWriteTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`JsonWrite<T>`]**: Node that writes data to JSON files while passing data through
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::JsonWrite;
//! use serde_json::Value;
//!
//! // Create a JSON write node
//! let json_write = JsonWrite::<Value>::new("output.json");
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::JsonWrite;
//! use streamweave::ErrorStrategy;
//! use serde_json::Value;
//!
//! // Create a JSON write node with error handling
//! let json_write = JsonWrite::<Value>::new("output.json")
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("json-writer".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Pass-Through Pattern**: Writes data while passing it through for
//!   intermediate result capture
//! - **Async I/O**: Uses Tokio's async filesystem operations for non-blocking
//!   file writing
//! - **JSON Serialization**: Uses serde_json for robust JSON serialization
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`JsonWrite`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::JsonWriteTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde::Serialize;
use std::path::PathBuf;
use std::pin::Pin;

/// Node that writes data to JSON files while passing data through.
///
/// This node wraps `JsonWriteTransformer` for use in graphs. It takes data as input,
/// writes it to a JSON file, and outputs the same data, enabling writing intermediate
/// results while continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{JsonWrite, TransformerNode};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Record {
///     name: String,
///     age: u32,
/// }
///
/// let json_write = JsonWrite::<Record>::new("output.json");
/// let node = TransformerNode::from_transformer(
///     "json_write".to_string(),
///     json_write,
/// );
/// ```
pub struct JsonWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying JSON write transformer
  transformer: JsonWriteTransformer<T>,
}

impl<T> JsonWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `JsonWrite` node for the specified file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the JSON file to write.
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      transformer: JsonWriteTransformer::new(path),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }

  /// Sets whether to write items as a JSON array.
  ///
  /// # Arguments
  ///
  /// * `as_array` - Whether to write items as a JSON array.
  pub fn with_as_array(mut self, as_array: bool) -> Self {
    self.transformer = self.transformer.with_as_array(as_array);
    self
  }

  /// Sets whether to pretty-print JSON.
  ///
  /// # Arguments
  ///
  /// * `pretty` - Whether to pretty-print JSON.
  pub fn with_pretty(mut self, pretty: bool) -> Self {
    self.transformer = self.transformer.with_pretty(pretty);
    self
  }

  /// Sets the buffer size.
  ///
  /// # Arguments
  ///
  /// * `size` - The buffer size.
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.transformer = self.transformer.with_buffer_size(size);
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &PathBuf {
    self.transformer.path()
  }
}

impl<T> Clone for JsonWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for JsonWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for JsonWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for JsonWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}

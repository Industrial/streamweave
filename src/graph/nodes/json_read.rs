//! JSON read node for reading JSON files in graphs.
//!
//! This module provides [`JsonRead`], a graph node that reads JSON files from file
//! paths. It takes file paths as input and outputs parsed JSON objects, enabling
//! processing of multiple JSON files in graph-based pipelines. It wraps
//! [`JsonReadTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`JsonRead`] is useful for reading JSON files in graph-based pipelines. It
//! supports reading multiple JSON files from file paths, making it ideal for
//! batch processing of JSON data files.
//!
//! # Key Concepts
//!
//! - **File Reading**: Reads JSON files from file paths
//! - **JSON Parsing**: Parses file contents as JSON
//! - **Path Input**: Takes file path strings as input
//! - **Transformer Wrapper**: Wraps `JsonReadTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`JsonRead`]**: Node that reads JSON files from file paths
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::JsonRead;
//!
//! // Create a JSON read node
//! let json_read = JsonRead::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::JsonRead;
//! use streamweave::ErrorStrategy;
//!
//! // Create a JSON read node with error handling
//! let json_read = JsonRead::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("json-reader".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Async I/O**: Uses Tokio's async filesystem operations for non-blocking
//!   file reading
//! - **JSON Parsing**: Uses serde_json for robust JSON parsing
//! - **Path Input**: Takes file paths as input for processing multiple files
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`JsonRead`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::JsonReadTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde::de::DeserializeOwned;
use std::pin::Pin;

/// Node that reads JSON files from input paths.
///
/// This node wraps `JsonReadTransformer` for use in graphs. It takes file paths
/// (String) as input and outputs deserialized JSON objects (T), enabling processing
/// of multiple JSON files in a pipeline.
///
/// This node can handle:
/// - JSON objects: yields the object as a single item
/// - JSON arrays: can yield each element as a separate item (if `array_as_stream` is true)
/// - JSON primitives: yields the value as a single item
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{JsonRead, TransformerNode};
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Clone, Debug)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let json_read = JsonRead::<Event>::new();
/// let node = TransformerNode::from_transformer(
///     "json_read".to_string(),
///     json_read,
/// );
/// ```
pub struct JsonRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// The underlying JSON read transformer
  transformer: JsonReadTransformer<T>,
}

impl<T> JsonRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Creates a new `JsonRead` node with default configuration.
  pub fn new() -> Self {
    Self {
      transformer: JsonReadTransformer::new(),
    }
  }

  /// Sets whether to treat JSON arrays as a stream of items.
  ///
  /// When true, if the JSON file contains an array, each element will be yielded as a separate item.
  /// When false, the entire array will be yielded as a single item.
  ///
  /// # Arguments
  ///
  /// * `array_as_stream` - Whether to treat JSON arrays as a stream of items.
  pub fn with_array_as_stream(mut self, array_as_stream: bool) -> Self {
    self.transformer = self.transformer.with_array_as_stream(array_as_stream);
    self
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
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
}

impl<T> Default for JsonRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for JsonRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for JsonRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl<T> Output for JsonRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for JsonRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type InputPorts = (String,);
  type OutputPorts = (T,);

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

//! JSONL read node for reading JSON Lines files in graphs.
//!
//! This module provides [`JsonlRead`], a graph node that reads JSON Lines (JSONL)
//! files from file paths. It takes file paths as input and outputs deserialized
//! JSONL objects, enabling processing of multiple JSONL files in graph-based pipelines.
//! It wraps [`JsonlReadTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`JsonlRead`] is useful for reading JSON Lines files in graph-based pipelines.
//! JSON Lines is a text format where each line is a valid JSON value. This node
//! reads each line from a file and deserializes it as the specified type, making
//! it ideal for processing large JSON datasets that are stored line-by-line.
//!
//! # Key Concepts
//!
//! - **JSON Lines Format**: Reads files where each line is a valid JSON value
//! - **Type-Safe Deserialization**: Deserializes JSONL lines into typed structures
//! - **File Path Input**: Takes file paths as input for processing multiple files
//! - **Transformer Wrapper**: Wraps `JsonlReadTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`JsonlRead<T>`]**: Node that reads JSONL files from file paths
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::JsonlRead;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize, Clone, Debug)]
//! struct Event {
//!     id: u32,
//!     message: String,
//! }
//!
//! // Create a JSONL read node
//! let jsonl_read = JsonlRead::<Event>::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::JsonlRead;
//! use streamweave::ErrorStrategy;
//! use serde::Deserialize;
//!
//! # #[derive(Deserialize, Clone, Debug)]
//! # struct Event { id: u32, message: String }
//! // Create a JSONL read node with error handling
//! let jsonl_read = JsonlRead::<Event>::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("jsonl-reader".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Lines Format**: Supports the JSON Lines format for efficient
//!   processing of large JSON datasets
//! - **Type-Safe Deserialization**: Uses Serde for type-safe deserialization
//!   into typed structures
//! - **File Path Input**: Takes file paths as input for batch processing
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`JsonlRead`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::JsonlReadTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde::de::DeserializeOwned;
use std::pin::Pin;

/// Node that reads JSONL files from input paths.
///
/// This node wraps `JsonlReadTransformer` for use in graphs. It takes file paths
/// (String) as input and outputs deserialized JSONL objects (T), enabling processing
/// of multiple JSONL files in a pipeline.
///
/// JSON Lines is a text format where each line is a valid JSON value.
/// This node reads each line from a file and deserializes it as the specified type.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{JsonlRead, TransformerNode};
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Clone, Debug)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let jsonl_read = JsonlRead::<Event>::new();
/// let node = TransformerNode::from_transformer(
///     "jsonl_read".to_string(),
///     jsonl_read,
/// );
/// ```
pub struct JsonlRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// The underlying JSONL read transformer
  transformer: JsonlReadTransformer<T>,
}

impl<T> JsonlRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Creates a new `JsonlRead` node with default configuration.
  pub fn new() -> Self {
    Self {
      transformer: JsonlReadTransformer::new(),
    }
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

impl<T> Default for JsonlRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for JsonlRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for JsonlRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl<T> Output for JsonlRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for JsonlRead<T>
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

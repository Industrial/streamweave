//! JSONL write node for writing data to JSON Lines files while passing data through.
//!
//! This module provides [`JsonlWrite`], a graph node that writes data to JSON Lines
//! (JSONL) files while passing the same data through to the output. It takes data as
//! input, writes it to a JSONL file, and outputs the same data, enabling writing
//! intermediate results while continuing processing. It wraps [`JsonlWriteTransformer`]
//! for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`JsonlWrite`] is useful for writing intermediate results to JSON Lines files while
//! continuing processing in graph-based pipelines. Unlike consumers, it passes data
//! through, making it ideal for checkpointing data at intermediate stages or logging
//! JSONL writes.
//!
//! # Key Concepts
//!
//! - **Pass-Through Operation**: Writes data to JSONL files while passing it through to output
//! - **JSON Lines Format**: Writes data in JSON Lines format (one JSON object per line)
//! - **Intermediate Results**: Enables writing intermediate results without
//!   interrupting the pipeline
//! - **Transformer Wrapper**: Wraps `JsonlWriteTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`JsonlWrite<T>`]**: Node that writes data to JSONL files while passing data through
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::JsonlWrite;
//! use serde::Serialize;
//!
//! #[derive(Serialize, Clone, Debug)]
//! struct Record {
//!     name: String,
//!     age: u32,
//! }
//!
//! // Create a JSONL write node
//! let jsonl_write = JsonlWrite::<Record>::new("output.jsonl");
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::JsonlWrite;
//! use streamweave::ErrorStrategy;
//! use serde::Serialize;
//!
//! # #[derive(Serialize, Clone, Debug)]
//! # struct Record { name: String, age: u32 }
//! // Create a JSONL write node with error handling
//! let jsonl_write = JsonlWrite::<Record>::new("output.jsonl")
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("jsonl-writer".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Pass-Through Pattern**: Writes data while passing it through for
//!   intermediate result capture
//! - **JSON Lines Format**: Supports the JSON Lines format for efficient
//!   storage of large JSON datasets
//! - **Async I/O**: Uses Tokio's async filesystem operations for non-blocking
//!   file writing
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`JsonlWrite`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::JsonlWriteTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde::Serialize;
use std::path::PathBuf;
use std::pin::Pin;

/// Node that writes data to JSON Lines (JSONL) files while passing data through.
///
/// This node wraps `JsonlWriteTransformer` for use in graphs. It takes data as input,
/// writes it to a JSONL file, and outputs the same data, enabling writing intermediate
/// results while continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{JsonlWrite, TransformerNode};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Record {
///     name: String,
///     age: u32,
/// }
///
/// let jsonl_write = JsonlWrite::<Record>::new("output.jsonl");
/// let node = TransformerNode::from_transformer(
///     "jsonl_write".to_string(),
///     jsonl_write,
/// );
/// ```
pub struct JsonlWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying JSONL write transformer
  transformer: JsonlWriteTransformer<T>,
}

impl<T> JsonlWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `JsonlWrite` node for the specified file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the JSONL file to write.
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      transformer: JsonlWriteTransformer::new(path),
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

  /// Sets whether to append to existing file.
  ///
  /// # Arguments
  ///
  /// * `append` - Whether to append to existing file.
  pub fn with_append(mut self, append: bool) -> Self {
    self.transformer = self.transformer.with_append(append);
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

impl<T> Clone for JsonlWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for JsonlWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for JsonlWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for JsonlWrite<T>
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

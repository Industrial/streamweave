//! CSV write node for writing data to CSV files while passing data through.
//!
//! This module provides [`CsvWrite`], a graph node that writes data to CSV files
//! while passing the same data through to the output. It takes data as input,
//! writes it to a CSV file, and outputs the same data, enabling writing intermediate
//! results while continuing processing. It wraps [`CsvWriteTransformer`] for use in
//! StreamWeave graphs.
//!
//! # Overview
//!
//! [`CsvWrite`] is useful for writing intermediate results to CSV files while
//! continuing processing in graph-based pipelines. Unlike consumers, it passes data
//! through, making it ideal for debugging, logging, or checkpointing data at
//! intermediate stages.
//!
//! # Key Concepts
//!
//! - **Pass-Through Operation**: Writes data to CSV while passing it through to output
//! - **Intermediate Results**: Enables writing intermediate results without
//!   interrupting the pipeline
//! - **Serializable Data**: Data must implement `Serialize` for CSV conversion
//! - **Transformer Wrapper**: Wraps `CsvWriteTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`CsvWrite<T>`]**: Node that writes data to CSV files while passing data through
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::CsvWrite;
//! use serde::Serialize;
//!
//! #[derive(Serialize, Clone, Debug)]
//! struct Record {
//!     name: String,
//!     age: u32,
//! }
//!
//! // Create a CSV write node
//! let csv_write = CsvWrite::<Record>::new("output.csv");
//! ```
//!
//! ## With Configuration
//!
//! ```rust
//! use streamweave::graph::nodes::CsvWrite;
//! use serde::Serialize;
//!
//! # #[derive(Serialize, Clone, Debug)]
//! # struct Record { name: String, age: u32 }
//! // Create a CSV write node with custom configuration
//! let csv_write = CsvWrite::<Record>::new("output.csv")
//!     .with_headers(true)           // Include header row
//!     .with_delimiter(b',')         // Comma delimiter
//!     .with_flush_on_write(true);   // Flush after each write
//! ```
//!
//! # Design Decisions
//!
//! - **Pass-Through Pattern**: Writes data while passing it through for
//!   intermediate result capture
//! - **CSV Library Integration**: Uses the `csv` crate for robust CSV writing
//! - **Serializable Data**: Requires `Serialize` trait for flexible data structure
//!   support
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`CsvWrite`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::CsvWriteTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde::Serialize;
use std::path::PathBuf;
use std::pin::Pin;

/// Node that writes data to CSV files while passing data through.
///
/// This node wraps `CsvWriteTransformer` for use in graphs. It takes data as input,
/// writes it to a CSV file, and outputs the same data, enabling writing intermediate
/// results while continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{CsvWrite, TransformerNode};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Record {
///     name: String,
///     age: u32,
/// }
///
/// let csv_write = CsvWrite::<Record>::new("output.csv");
/// let node = TransformerNode::from_transformer(
///     "csv_write".to_string(),
///     csv_write,
/// );
/// ```
pub struct CsvWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying CSV write transformer
  transformer: CsvWriteTransformer<T>,
}

impl<T> CsvWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `CsvWrite` node for the specified file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the CSV file to write.
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      transformer: CsvWriteTransformer::new(path),
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

  /// Sets whether to write a header row.
  ///
  /// # Arguments
  ///
  /// * `write_headers` - Whether to write a header row.
  pub fn with_headers(mut self, write_headers: bool) -> Self {
    self.transformer = self.transformer.with_headers(write_headers);
    self
  }

  /// Sets the delimiter character.
  ///
  /// # Arguments
  ///
  /// * `delimiter` - The delimiter character.
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.transformer = self.transformer.with_delimiter(delimiter);
    self
  }

  /// Sets whether to flush after each record.
  ///
  /// # Arguments
  ///
  /// * `flush` - Whether to flush after each record.
  pub fn with_flush_on_write(mut self, flush: bool) -> Self {
    self.transformer = self.transformer.with_flush_on_write(flush);
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &PathBuf {
    self.transformer.path()
  }
}

impl<T> Clone for CsvWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for CsvWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for CsvWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for CsvWrite<T>
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

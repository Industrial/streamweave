//! File write node for writing data to files while passing data through.
//!
//! This module provides [`FsFileWrite`], a graph node that writes string data to files
//! while passing the same data through to the output. It takes string data as input,
//! writes it to a file, and outputs the same data, enabling writing intermediate results
//! while continuing processing. It wraps [`FsFileWriteTransformer`] for use in
//! StreamWeave graphs.
//!
//! # Overview
//!
//! [`FsFileWrite`] is useful for writing intermediate results to files while continuing
//! processing in graph-based pipelines. Unlike consumers, it passes data through, making
//! it ideal for checkpointing data at intermediate stages or logging file writes.
//!
//! # Key Concepts
//!
//! - **Pass-Through Operation**: Writes data to files while passing it through to output
//! - **String Data**: Takes string data as input and writes it to files
//! - **Intermediate Results**: Enables writing intermediate results without
//!   interrupting the pipeline
//! - **Transformer Wrapper**: Wraps `FsFileWriteTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`FsFileWrite`]**: Node that writes data to files while passing data through
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::FsFileWrite;
//!
//! // Create a file write node
//! let fs_file_write = FsFileWrite::new("output.txt");
//! ```
//!
//! ## With Configuration
//!
//! ```rust
//! use streamweave::graph::nodes::FsFileWrite;
//!
//! // Create a file write node with configuration
//! let fs_file_write = FsFileWrite::new("output.txt")
//!     .with_append(true)         // Append to existing file
//!     .with_buffer_size(16384);  // 16KB buffer
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::FsFileWrite;
//! use streamweave::ErrorStrategy;
//!
//! // Create a file write node with error handling
//! let fs_file_write = FsFileWrite::new("output.txt")
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("file-writer".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Pass-Through Pattern**: Writes data while passing it through for
//!   intermediate result capture
//! - **Async I/O**: Uses Tokio's async filesystem operations for non-blocking
//!   file writing
//! - **Configurable Buffering**: Supports configurable buffer sizes for performance
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`FsFileWrite`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::FsFileWriteTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::path::PathBuf;
use std::pin::Pin;

/// Node that writes string data to files while passing data through.
///
/// This node wraps `FsFileWriteTransformer` for use in graphs. It takes string data as input,
/// writes it to a file, and outputs the same data, enabling writing intermediate
/// results while continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{FsFileWrite, TransformerNode};
///
/// let fs_file_write = FsFileWrite::new("output.txt");
/// let node = TransformerNode::from_transformer(
///     "fs_file_write".to_string(),
///     fs_file_write,
/// );
/// ```
pub struct FsFileWrite {
  /// The underlying file write transformer
  transformer: FsFileWriteTransformer,
}

impl FsFileWrite {
  /// Creates a new `FsFileWrite` node for the specified file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the file to write.
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      transformer: FsFileWriteTransformer::new(path),
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

impl Clone for FsFileWrite {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for FsFileWrite {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for FsFileWrite {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for FsFileWrite {
  type InputPorts = (String,);
  type OutputPorts = (String,);

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

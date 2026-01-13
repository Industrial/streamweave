//! Directory list node for listing directory entries in graphs.
//!
//! This module provides [`FsDirectoryList`], a graph node that lists directory entries
//! from input paths. It takes directory paths as input and outputs file/directory paths,
//! enabling processing of multiple directories in graph-based pipelines. It wraps
//! [`FsDirectoryListTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`FsDirectoryList`] is useful for listing directory contents in graph-based pipelines.
//! It supports both recursive and non-recursive directory traversal, making it ideal
//! for file processing workflows that need to discover files in directory structures.
//!
//! # Key Concepts
//!
//! - **Directory Traversal**: Lists entries in directories (files and subdirectories)
//! - **Recursive Support**: Supports both recursive and non-recursive traversal
//! - **Path Output**: Outputs file/directory paths as strings
//! - **Transformer Wrapper**: Wraps `FsDirectoryListTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`FsDirectoryList`]**: Node that lists directory entries from input paths
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::FsDirectoryList;
//!
//! // Create a directory list node (non-recursive)
//! let fs_directory_list = FsDirectoryList::new();
//! ```
//!
//! ## With Recursive Traversal
//!
//! ```rust
//! use streamweave::graph::nodes::FsDirectoryList;
//!
//! // Create a directory list node with recursive traversal
//! let fs_directory_list = FsDirectoryList::new()
//!     .with_recursive(true);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::FsDirectoryList;
//! use streamweave::ErrorStrategy;
//!
//! // Create a directory list node with error handling
//! let fs_directory_list = FsDirectoryList::new()
//!     .with_recursive(true)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("dir-lister".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Async I/O**: Uses Tokio's async filesystem operations for non-blocking
//!   directory listing
//! - **Recursive Option**: Supports both recursive and non-recursive traversal
//!   for flexibility
//! - **Path Strings**: Works with path strings for simplicity and compatibility
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`FsDirectoryList`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::FsDirectoryListTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that lists directory entries from input paths.
///
/// This node wraps `FsDirectoryListTransformer` for use in graphs. It takes directory paths
/// (String) as input and outputs file/directory paths (String), enabling processing
/// of multiple directories in a pipeline.
///
/// This node supports both recursive and non-recursive directory traversal.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{FsDirectoryList, TransformerNode};
///
/// // Non-recursive listing
/// let fs_directory_list = FsDirectoryList::new();
/// let node = TransformerNode::from_transformer(
///     "fs_directory_list".to_string(),
///     fs_directory_list,
/// );
///
/// // Recursive listing
/// let fs_directory_list = FsDirectoryList::new()
///     .with_recursive(true);
/// ```
pub struct FsDirectoryList {
  /// The underlying directory list transformer
  transformer: FsDirectoryListTransformer,
}

impl FsDirectoryList {
  /// Creates a new `FsDirectoryList` node with default configuration (non-recursive).
  pub fn new() -> Self {
    Self {
      transformer: FsDirectoryListTransformer::new(),
    }
  }

  /// Sets whether to read directories recursively.
  ///
  /// When true, recursively walks through subdirectories.
  /// When false, only lists entries in the top-level directory.
  ///
  /// # Arguments
  ///
  /// * `recursive` - Whether to read directories recursively.
  pub fn with_recursive(mut self, recursive: bool) -> Self {
    self.transformer = self.transformer.with_recursive(recursive);
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

impl Default for FsDirectoryList {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for FsDirectoryList {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for FsDirectoryList {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for FsDirectoryList {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for FsDirectoryList {
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

//! Directory create node for creating directories while passing paths through.
//!
//! This module provides [`FsDirectoryCreate`], a graph node that creates directories
//! from path strings while passing the same paths through to the output. It takes
//! path strings as input, creates directories, and outputs the same paths, enabling
//! creating directories and continuing processing. It wraps [`FsDirectoryCreateTransformer`]
//! for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`FsDirectoryCreate`] is useful for creating directory structures while continuing
//! processing in graph-based pipelines. Unlike consumers, it passes paths through,
//! making it ideal for setting up directory structures at intermediate stages
//! without interrupting the pipeline.
//!
//! # Key Concepts
//!
//! - **Pass-Through Operation**: Creates directories while passing paths through to output
//! - **Path Input**: Takes directory path strings as input
//! - **Recursive Creation**: Creates parent directories as needed
//! - **Transformer Wrapper**: Wraps `FsDirectoryCreateTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`FsDirectoryCreate`]**: Node that creates directories while passing paths through
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::FsDirectoryCreate;
//!
//! // Create a directory create node
//! let fs_directory_create = FsDirectoryCreate::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::FsDirectoryCreate;
//! use streamweave::ErrorStrategy;
//!
//! // Create a directory create node with error handling
//! let fs_directory_create = FsDirectoryCreate::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("dir-creator".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Pass-Through Pattern**: Creates directories while passing paths through for
//!   intermediate directory setup
//! - **Async I/O**: Uses Tokio's async filesystem operations for non-blocking
//!   directory creation
//! - **Recursive Creation**: Creates parent directories automatically for convenience
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`FsDirectoryCreate`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::FsDirectoryCreateTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that creates directories from path strings while passing paths through.
///
/// This node wraps `FsDirectoryCreateTransformer` for use in graphs. It takes path strings as input,
/// creates directories, and outputs the same paths, enabling creating directories and continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{FsDirectoryCreate, TransformerNode};
///
/// let fs_directory_create = FsDirectoryCreate::new();
/// let node = TransformerNode::from_transformer(
///     "fs_directory_create".to_string(),
///     fs_directory_create,
/// );
/// ```
pub struct FsDirectoryCreate {
  /// The underlying directory create transformer
  transformer: FsDirectoryCreateTransformer,
}

impl FsDirectoryCreate {
  /// Creates a new `FsDirectoryCreate` node.
  pub fn new() -> Self {
    Self {
      transformer: FsDirectoryCreateTransformer::new(),
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

impl Default for FsDirectoryCreate {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for FsDirectoryCreate {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for FsDirectoryCreate {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for FsDirectoryCreate {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for FsDirectoryCreate {
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

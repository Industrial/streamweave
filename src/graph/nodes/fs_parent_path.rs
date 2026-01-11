//! Parent path extraction node for extracting parent directories from paths.
//!
//! This module provides [`FsParentPath`], a graph node that extracts the parent
//! directory path from file path strings. It wraps [`FsParentPathTransformer`] for
//! use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`FsParentPath`] is useful for extracting parent directory paths from file paths
//! in graph-based pipelines. It processes path strings and extracts the parent
//! directory component, making it ideal for file processing workflows that need
//! to work with directory paths.
//!
//! # Key Concepts
//!
//! - **Parent Extraction**: Extracts the parent directory from file paths
//! - **Path Parsing**: Parses file paths and extracts the directory component
//! - **String Transformation**: Converts full paths to parent directory paths
//! - **Transformer Wrapper**: Wraps `FsParentPathTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`FsParentPath`]**: Node that extracts parent directory paths from file paths
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::FsParentPath;
//!
//! // Create a parent path extraction node
//! let parent_path = FsParentPath::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::FsParentPath;
//! use streamweave::ErrorStrategy;
//!
//! // Create a parent path extraction node with error handling
//! let parent_path = FsParentPath::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("parent-extractor".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Path Library Integration**: Uses Rust's standard path handling for
//!   cross-platform compatibility
//! - **Parent Extraction**: Extracts just the parent directory component,
//!   not the filename
//! - **String-Based**: Works with path strings for simplicity and compatibility
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`FsParentPath`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::FsParentPathTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that extracts the parent directory path from file path strings.
///
/// This node wraps `FsParentPathTransformer` for use in graphs. It extracts
/// the parent directory component from path strings.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{FsParentPath, TransformerNode};
///
/// let parent_path = FsParentPath::new();
/// let node = TransformerNode::from_transformer(
///     "parent_path".to_string(),
///     parent_path,
/// );
/// ```
pub struct FsParentPath {
  /// The underlying parent path transformer
  transformer: FsParentPathTransformer,
}

impl FsParentPath {
  /// Creates a new `FsParentPath` node.
  pub fn new() -> Self {
    Self {
      transformer: FsParentPathTransformer::new(),
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

impl Default for FsParentPath {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for FsParentPath {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for FsParentPath {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for FsParentPath {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for FsParentPath {
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

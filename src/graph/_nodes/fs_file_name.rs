//! File name extraction node for extracting filenames from paths.
//!
//! This module provides [`FsFileName`], a graph node that extracts the file name
//! component from file path strings. It wraps [`FsFileNameTransformer`] for use in
//! StreamWeave graphs. This is useful for processing file paths and extracting just
//! the filename component.
//!
//! # Overview
//!
//! [`FsFileName`] is useful for extracting filenames from full file paths in
//! graph-based pipelines. It processes path strings and extracts the filename
//! component, making it ideal for file processing workflows that need to work
//! with just filenames.
//!
//! # Key Concepts
//!
//! - **Path Parsing**: Parses file paths and extracts the filename component
//! - **String Transformation**: Converts full paths to filenames
//! - **Path Handling**: Handles various path formats (Unix, Windows, relative, absolute)
//! - **Transformer Wrapper**: Wraps `FsFileNameTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`FsFileName`]**: Node that extracts filenames from path strings
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::FsFileName;
//!
//! // Create a file name extraction node
//! let fs_file_name = FsFileName::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::FsFileName;
//! use streamweave::ErrorStrategy;
//!
//! // Create a file name extraction node with error handling
//! let fs_file_name = FsFileName::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("filename-extractor".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Path Library Integration**: Uses Rust's standard path handling for
//!   cross-platform compatibility
//! - **Filename Extraction**: Extracts just the filename component, not the
//!   directory path
//! - **String-Based**: Works with path strings for simplicity and compatibility
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`FsFileName`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::FsFileNameTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that extracts the file name from file path strings.
///
/// This node wraps `FsFileNameTransformer` for use in graphs. It extracts the
/// file name component from path strings.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{FsFileName, TransformerNode};
///
/// let fs_file_name = FsFileName::new();
/// let node = TransformerNode::from_transformer(
///     "extract_filename".to_string(),
///     fs_file_name,
/// );
/// ```
pub struct FsFileName {
  /// The underlying file name transformer
  transformer: FsFileNameTransformer,
}

impl FsFileName {
  /// Creates a new `FsFileName` node.
  pub fn new() -> Self {
    Self {
      transformer: FsFileNameTransformer::new(),
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

impl Default for FsFileName {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for FsFileName {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for FsFileName {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for FsFileName {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for FsFileName {
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

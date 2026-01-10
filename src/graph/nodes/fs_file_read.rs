//! File read node for StreamWeave graphs
//!
//! Reads file contents from file paths. Takes file paths as input and outputs
//! file contents (lines or bytes), enabling processing of multiple files in a pipeline.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{FileContent, FsFileReadTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that reads file contents from input paths.
///
/// This node wraps `FsFileReadTransformer` for use in graphs. It takes file paths
/// (String) as input and outputs file contents (FileContent), enabling processing
/// of multiple files in a pipeline.
///
/// This node can read files in two modes:
/// - Lines mode: Reads the file line by line, emitting each line as FileContent::Line
/// - Bytes mode: Reads the file as raw bytes, emitting chunks as FileContent::Bytes
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{FsFileRead, TransformerNode};
///
/// // Read as lines
/// let fs_file_read = FsFileRead::new();
/// let node = TransformerNode::from_transformer(
///     "fs_file_read".to_string(),
///     fs_file_read,
/// );
///
/// // Read as bytes
/// let fs_file_read = FsFileRead::new()
///     .with_read_as_lines(false);
/// ```
pub struct FsFileRead {
  /// The underlying file read transformer
  transformer: FsFileReadTransformer,
}

impl FsFileRead {
  /// Creates a new `FsFileRead` node with default configuration (reads as lines).
  pub fn new() -> Self {
    Self {
      transformer: FsFileReadTransformer::new(),
    }
  }

  /// Sets whether to read as lines.
  ///
  /// When true, reads the file line by line, emitting each line as FileContent::Line.
  /// When false, reads the file as raw bytes, emitting chunks as FileContent::Bytes.
  ///
  /// # Arguments
  ///
  /// * `read_as_lines` - Whether to read as lines.
  pub fn with_read_as_lines(mut self, read_as_lines: bool) -> Self {
    self.transformer = self.transformer.with_read_as_lines(read_as_lines);
    self
  }

  /// Sets the buffer size for reading bytes.
  ///
  /// Only used when read_as_lines is false.
  ///
  /// # Arguments
  ///
  /// * `buffer_size` - Buffer size in bytes.
  pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
    self.transformer = self.transformer.with_buffer_size(buffer_size);
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

impl Default for FsFileRead {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for FsFileRead {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for FsFileRead {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for FsFileRead {
  type Output = FileContent;
  type OutputStream = Pin<Box<dyn Stream<Item = FileContent> + Send>>;
}

#[async_trait]
impl Transformer for FsFileRead {
  type InputPorts = (String,);
  type OutputPorts = (FileContent,);

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

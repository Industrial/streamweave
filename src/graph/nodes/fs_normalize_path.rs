//! Path normalization node for StreamWeave graphs
//!
//! Normalizes file path strings by removing redundant separators and resolving
//! `.` and `..` components where possible.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::FsNormalizePathTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that normalizes file path strings.
///
/// This node wraps `FsNormalizePathTransformer` for use in graphs. It normalizes
/// paths by removing redundant separators and resolving `.` and `..` components.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{FsNormalizePath, TransformerNode};
///
/// let normalize = FsNormalizePath::new();
/// let node = TransformerNode::from_transformer(
///     "normalize".to_string(),
///     normalize,
/// );
/// ```
pub struct FsNormalizePath {
  /// The underlying path normalization transformer
  transformer: FsNormalizePathTransformer,
}

impl FsNormalizePath {
  /// Creates a new `FsNormalizePath` node.
  pub fn new() -> Self {
    Self {
      transformer: FsNormalizePathTransformer::new(),
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

impl Default for FsNormalizePath {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for FsNormalizePath {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for FsNormalizePath {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for FsNormalizePath {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for FsNormalizePath {
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

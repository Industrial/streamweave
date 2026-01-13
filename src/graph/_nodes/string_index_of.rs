//! String index of node for StreamWeave graphs
//!
//! Finds the index of a substring in strings, producing a stream of `Option<usize>` values.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{IndexOfMode, StringIndexOfTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that finds the index of a substring in strings.
///
/// This node wraps `StringIndexOfTransformer` for use in graphs.
pub struct StringIndexOf {
  /// The underlying index of transformer
  transformer: StringIndexOfTransformer,
}

impl StringIndexOf {
  /// Creates a new `StringIndexOf` node.
  ///
  /// # Arguments
  ///
  /// * `substring` - The substring to find.
  /// * `mode` - The search mode (First or Last).
  pub fn new(substring: impl Into<String>, mode: IndexOfMode) -> Self {
    Self {
      transformer: StringIndexOfTransformer::new(substring, mode),
    }
  }

  /// Sets the error handling strategy for this node.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl Clone for StringIndexOf {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for StringIndexOf {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringIndexOf {
  type Output = Option<usize>;
  type OutputStream = Pin<Box<dyn Stream<Item = Option<usize>> + Send>>;
}

#[async_trait]
impl Transformer for StringIndexOf {
  type InputPorts = (String,);
  type OutputPorts = (Option<usize>,);

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

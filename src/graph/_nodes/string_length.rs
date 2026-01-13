//! String length node for StreamWeave graphs
//!
//! Gets the length of strings, producing a stream of usize values.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::StringLengthTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that gets the length of strings.
///
/// This node wraps `StringLengthTransformer` for use in graphs.
pub struct StringLength {
  /// The underlying length transformer
  transformer: StringLengthTransformer,
}

impl StringLength {
  /// Creates a new `StringLength` node.
  pub fn new() -> Self {
    Self {
      transformer: StringLengthTransformer::new(),
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

impl Default for StringLength {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for StringLength {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for StringLength {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringLength {
  type Output = usize;
  type OutputStream = Pin<Box<dyn Stream<Item = usize> + Send>>;
}

#[async_trait]
impl Transformer for StringLength {
  type InputPorts = (String,);
  type OutputPorts = (usize,);

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

//! String concat node for StreamWeave graphs
//!
//! Concatenates multiple strings into a single string.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::StringConcatTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that concatenates strings.
///
/// This node wraps `StringConcatTransformer` for use in graphs.
pub struct StringConcat {
  /// The underlying concat transformer
  transformer: StringConcatTransformer,
}

impl StringConcat {
  /// Creates a new `StringConcat` node.
  pub fn new() -> Self {
    Self {
      transformer: StringConcatTransformer::new(),
    }
  }

  /// Sets the error handling strategy for this node.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Vec<String>>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl Default for StringConcat {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for StringConcat {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for StringConcat {
  type Input = Vec<String>;
  type InputStream = Pin<Box<dyn Stream<Item = Vec<String>> + Send>>;
}

impl Output for StringConcat {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringConcat {
  type InputPorts = (Vec<String>,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Vec<String>>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<Vec<String>> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Vec<String>> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<Vec<String>>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<Vec<String>>) -> ErrorContext<Vec<String>> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}

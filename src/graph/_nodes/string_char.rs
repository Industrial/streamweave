//! String char node for StreamWeave graphs
//!
//! Performs character operations on strings (char_at, char_code_at, from_char_code).

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{CharOperation, StringCharTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that performs character operations on strings.
///
/// This node wraps `StringCharTransformer` for use in graphs.
pub struct StringChar {
  /// The underlying char transformer
  transformer: StringCharTransformer,
}

impl StringChar {
  /// Creates a new `StringChar` node.
  ///
  /// # Arguments
  ///
  /// * `operation` - The character operation to perform.
  pub fn new(operation: CharOperation) -> Self {
    Self {
      transformer: StringCharTransformer::new(operation),
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

impl Clone for StringChar {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for StringChar {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringChar {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringChar {
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

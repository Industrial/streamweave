//! String repeat node for StreamWeave graphs
//!
//! Repeats strings a specified number of times.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::StringRepeatTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that repeats strings.
///
/// This node wraps `StringRepeatTransformer` for use in graphs.
pub struct StringRepeat {
  /// The underlying repeat transformer
  transformer: StringRepeatTransformer,
}

impl StringRepeat {
  /// Creates a new `StringRepeat` node.
  ///
  /// # Arguments
  ///
  /// * `count` - Number of times to repeat the string.
  pub fn new(count: usize) -> Self {
    Self {
      transformer: StringRepeatTransformer::new(count),
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

impl Clone for StringRepeat {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for StringRepeat {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringRepeat {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringRepeat {
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

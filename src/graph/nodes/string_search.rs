//! String search node for StreamWeave graphs
//!
//! Searches for regex patterns in strings, producing match information.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::StringSearchTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that searches for regex patterns in strings.
///
/// This node wraps `StringSearchTransformer` for use in graphs.
pub struct StringSearch {
  /// The underlying search transformer
  transformer: StringSearchTransformer,
}

impl StringSearch {
  /// Creates a new `StringSearch` node with the specified regex pattern.
  ///
  /// # Errors
  ///
  /// Returns an error if the pattern is invalid.
  pub fn new(pattern: &str) -> Result<Self, regex::Error> {
    Ok(Self {
      transformer: StringSearchTransformer::new(pattern)?,
    })
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

impl Clone for StringSearch {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for StringSearch {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringSearch {
  type Output = Option<String>;
  type OutputStream = Pin<Box<dyn Stream<Item = Option<String>> + Send>>;
}

#[async_trait]
impl Transformer for StringSearch {
  type InputPorts = (String,);
  type OutputPorts = (Option<String>,);

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

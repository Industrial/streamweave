//! XML stringify node for StreamWeave graphs
//!
//! Converts JSON values to XML strings.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::XmlStringifyTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that stringifies JSON values to XML from stream items.
///
/// This node wraps `XmlStringifyTransformer` for use in graphs.
pub struct XmlStringify {
  /// The underlying stringify transformer
  transformer: XmlStringifyTransformer,
}

impl XmlStringify {
  /// Creates a new `XmlStringify` node.
  pub fn new() -> Self {
    Self {
      transformer: XmlStringifyTransformer::new(),
    }
  }

  /// Sets whether to pretty-print XML.
  pub fn with_pretty(mut self, pretty: bool) -> Self {
    self.transformer = self.transformer.with_pretty(pretty);
    self
  }

  /// Sets the root element name.
  pub fn with_root_element(mut self, root: impl Into<String>) -> Self {
    self.transformer = self.transformer.with_root_element(root);
    self
  }

  /// Sets the error handling strategy for this node.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Value>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl Default for XmlStringify {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for XmlStringify {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for XmlStringify {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for XmlStringify {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for XmlStringify {
  type InputPorts = (Value,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Value>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<Value> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Value> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<Value>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<Value>) -> ErrorContext<Value> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}

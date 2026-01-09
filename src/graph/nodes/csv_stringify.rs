//! CSV stringify node for StreamWeave graphs
//!
//! Converts JSON objects/arrays to CSV strings.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::CsvStringifyTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that stringifies JSON values to CSV from stream items.
///
/// This node wraps `CsvStringifyTransformer` for use in graphs.
pub struct CsvStringify {
  /// The underlying stringify transformer
  transformer: CsvStringifyTransformer,
}

impl CsvStringify {
  /// Creates a new `CsvStringify` node.
  pub fn new() -> Self {
    Self {
      transformer: CsvStringifyTransformer::new(),
    }
  }

  /// Sets whether to write a header row.
  pub fn with_headers(mut self, write_headers: bool) -> Self {
    self.transformer = self.transformer.with_headers(write_headers);
    self
  }

  /// Sets the delimiter character.
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.transformer = self.transformer.with_delimiter(delimiter);
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

impl Default for CsvStringify {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for CsvStringify {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for CsvStringify {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for CsvStringify {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for CsvStringify {
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

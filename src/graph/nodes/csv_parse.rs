//! CSV parse node for StreamWeave graphs
//!
//! Parses CSV strings from stream items.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::CsvParseTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that parses CSV strings from stream items.
///
/// This node wraps `CsvParseTransformer` for use in graphs.
pub struct CsvParse {
  /// The underlying parse transformer
  transformer: CsvParseTransformer,
}

impl CsvParse {
  /// Creates a new `CsvParse` node.
  pub fn new() -> Self {
    Self {
      transformer: CsvParseTransformer::new(),
    }
  }

  /// Sets whether the CSV has a header row.
  pub fn with_headers(mut self, has_headers: bool) -> Self {
    self.transformer = self.transformer.with_headers(has_headers);
    self
  }

  /// Sets the delimiter character.
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.transformer = self.transformer.with_delimiter(delimiter);
    self
  }

  /// Sets whether to trim whitespace from fields.
  pub fn with_trim(mut self, trim: bool) -> Self {
    self.transformer = self.transformer.with_trim(trim);
    self
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

impl Default for CsvParse {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for CsvParse {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for CsvParse {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for CsvParse {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for CsvParse {
  type InputPorts = (String,);
  type OutputPorts = (Value,);

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

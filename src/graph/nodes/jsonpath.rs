//! JSONPath node for StreamWeave graphs
//!
//! Queries JSON objects using JSONPath expressions.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{JsonPathOperation, JsonPathTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that queries JSON objects using JSONPath expressions.
///
/// This node wraps `JsonPathTransformer` for use in graphs.
pub struct JsonPath {
  /// The underlying JSONPath transformer
  transformer: JsonPathTransformer,
}

impl JsonPath {
  /// Creates a new `JsonPath` node.
  ///
  /// # Arguments
  ///
  /// * `path` - The JSONPath expression (e.g., "$.name", "$.users\[0\].name").
  /// * `operation` - The operation to perform (Get or Compare).
  /// * `compare_value` - Optional value to compare against (for Compare operation).
  pub fn new(
    path: impl Into<String>,
    operation: JsonPathOperation,
    compare_value: Option<Value>,
  ) -> Self {
    Self {
      transformer: JsonPathTransformer::new(path, operation, compare_value),
    }
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

impl Clone for JsonPath {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for JsonPath {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for JsonPath {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for JsonPath {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

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

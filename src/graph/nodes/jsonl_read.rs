//! JSONL read node for StreamWeave graphs
//!
//! Reads JSONL files from file paths. Takes file paths as input and outputs
//! parsed JSONL objects, enabling processing of multiple JSONL files in a pipeline.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::JsonlReadTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde::de::DeserializeOwned;
use std::pin::Pin;

/// Node that reads JSONL files from input paths.
///
/// This node wraps `JsonlReadTransformer` for use in graphs. It takes file paths
/// (String) as input and outputs deserialized JSONL objects (T), enabling processing
/// of multiple JSONL files in a pipeline.
///
/// JSON Lines is a text format where each line is a valid JSON value.
/// This node reads each line from a file and deserializes it as the specified type.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{JsonlRead, TransformerNode};
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Clone, Debug)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let jsonl_read = JsonlRead::<Event>::new();
/// let node = TransformerNode::from_transformer(
///     "jsonl_read".to_string(),
///     jsonl_read,
/// );
/// ```
pub struct JsonlRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// The underlying JSONL read transformer
  transformer: JsonlReadTransformer<T>,
}

impl<T> JsonlRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Creates a new `JsonlRead` node with default configuration.
  pub fn new() -> Self {
    Self {
      transformer: JsonlReadTransformer::new(),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl<T> Default for JsonlRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for JsonlRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for JsonlRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl<T> Output for JsonlRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for JsonlRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type InputPorts = (String,);
  type OutputPorts = (T,);

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

//! JSON write node for StreamWeave graphs
//!
//! Writes data to JSON files while passing data through. Takes data as input, writes to JSON,
//! and outputs the same data, enabling writing intermediate results while continuing processing.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::JsonWriteTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde::Serialize;
use std::path::PathBuf;
use std::pin::Pin;

/// Node that writes data to JSON files while passing data through.
///
/// This node wraps `JsonWriteTransformer` for use in graphs. It takes data as input,
/// writes it to a JSON file, and outputs the same data, enabling writing intermediate
/// results while continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{JsonWrite, TransformerNode};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Record {
///     name: String,
///     age: u32,
/// }
///
/// let json_write = JsonWrite::<Record>::new("output.json");
/// let node = TransformerNode::from_transformer(
///     "json_write".to_string(),
///     json_write,
/// );
/// ```
pub struct JsonWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying JSON write transformer
  transformer: JsonWriteTransformer<T>,
}

impl<T> JsonWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `JsonWrite` node for the specified file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the JSON file to write.
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      transformer: JsonWriteTransformer::new(path),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
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

  /// Sets whether to write items as a JSON array.
  ///
  /// # Arguments
  ///
  /// * `as_array` - Whether to write items as a JSON array.
  pub fn with_as_array(mut self, as_array: bool) -> Self {
    self.transformer = self.transformer.with_as_array(as_array);
    self
  }

  /// Sets whether to pretty-print JSON.
  ///
  /// # Arguments
  ///
  /// * `pretty` - Whether to pretty-print JSON.
  pub fn with_pretty(mut self, pretty: bool) -> Self {
    self.transformer = self.transformer.with_pretty(pretty);
    self
  }

  /// Sets the buffer size.
  ///
  /// # Arguments
  ///
  /// * `size` - The buffer size.
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.transformer = self.transformer.with_buffer_size(size);
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &PathBuf {
    self.transformer.path()
  }
}

impl<T> Clone for JsonWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for JsonWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for JsonWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for JsonWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}

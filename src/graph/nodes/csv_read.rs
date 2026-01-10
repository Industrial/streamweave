//! CSV read node for StreamWeave graphs
//!
//! Reads CSV files from file paths. Takes file paths as input and outputs
//! parsed CSV rows, enabling processing of multiple CSV files in a pipeline.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::CsvReadTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde::de::DeserializeOwned;
use std::pin::Pin;

/// Node that reads CSV files from input paths.
///
/// This node wraps `CsvReadTransformer` for use in graphs. It takes file paths
/// (String) as input and outputs deserialized CSV rows (T), enabling processing
/// of multiple CSV files in a pipeline.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{CsvRead, TransformerNode};
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Clone, Debug)]
/// struct Record {
///     name: String,
///     age: u32,
/// }
///
/// let csv_read = CsvRead::<Record>::new();
/// let node = TransformerNode::from_transformer(
///     "csv_read".to_string(),
///     csv_read,
/// );
/// ```
pub struct CsvRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// The underlying CSV read transformer
  transformer: CsvReadTransformer<T>,
}

impl<T> CsvRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Creates a new `CsvRead` node with default configuration.
  pub fn new() -> Self {
    Self {
      transformer: CsvReadTransformer::new(),
    }
  }

  /// Sets whether the CSV has a header row.
  ///
  /// # Arguments
  ///
  /// * `has_headers` - Whether the CSV has a header row.
  pub fn with_headers(mut self, has_headers: bool) -> Self {
    self.transformer = self.transformer.with_headers(has_headers);
    self
  }

  /// Sets the delimiter character.
  ///
  /// # Arguments
  ///
  /// * `delimiter` - The delimiter character.
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.transformer = self.transformer.with_delimiter(delimiter);
    self
  }

  /// Sets whether to allow flexible column counts.
  ///
  /// # Arguments
  ///
  /// * `flexible` - Whether to allow flexible column counts.
  pub fn with_flexible(mut self, flexible: bool) -> Self {
    self.transformer = self.transformer.with_flexible(flexible);
    self
  }

  /// Sets whether to trim whitespace from fields.
  ///
  /// # Arguments
  ///
  /// * `trim` - Whether to trim whitespace from fields.
  pub fn with_trim(mut self, trim: bool) -> Self {
    self.transformer = self.transformer.with_trim(trim);
    self
  }

  /// Sets the comment character.
  ///
  /// # Arguments
  ///
  /// * `comment` - The comment character (None means no comments).
  pub fn with_comment(mut self, comment: Option<u8>) -> Self {
    self.transformer = self.transformer.with_comment(comment);
    self
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

impl<T> Default for CsvRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for CsvRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for CsvRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl<T> Output for CsvRead<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for CsvRead<T>
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

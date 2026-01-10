//! Zip node for StreamWeave graphs
//!
//! Zips items from multiple input streams into tuples. Collects items from
//! multiple input streams and combines them into vectors.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ZipTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that zips items from multiple input streams into tuples.
///
/// This node wraps `ZipTransformer` for use in graphs. It collects items from
/// multiple input streams and combines them into vectors, emitting one vector
/// per combination of items from each stream.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Zip, TransformerNode};
///
/// let zip = Zip::new();
/// let node = TransformerNode::from_transformer(
///     "zip".to_string(),
///     zip,
/// );
/// ```
pub struct Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying zip transformer
  transformer: ZipTransformer<T>,
}

impl<T> Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Zip` node.
  pub fn new() -> Self {
    Self {
      transformer: ZipTransformer::new(),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Vec<T>>) -> Self {
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

impl<T> Default for Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = Vec<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

impl<T> Output for Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

#[async_trait]
impl<T> Transformer for Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (Vec<T>,);
  type OutputPorts = (Vec<T>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Vec<T>>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<Vec<T>> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Vec<T>> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<Vec<T>>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<Vec<T>>) -> ErrorContext<Vec<T>> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}

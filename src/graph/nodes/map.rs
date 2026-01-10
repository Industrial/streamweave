//! Map node for StreamWeave graphs
//!
//! Applies a transformation function to each item in the stream, creating a
//! one-to-one mapping from input to output.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::MapTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that maps items using a transformation function.
///
/// This node wraps `MapTransformer` for use in graphs. It applies a function
/// to each input item to transform it into an output item.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Map, TransformerNode};
///
/// let map = Map::new(|x: i32| x * 2);
/// let node = TransformerNode::from_transformer(
///     "double".to_string(),
///     map,
/// );
/// ```
pub struct Map<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying map transformer
  transformer: MapTransformer<F, I, O>,
}

impl<F, I, O> Map<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Map` node with the specified transformation function.
  ///
  /// # Arguments
  ///
  /// * `f` - The function to apply to each input item.
  pub fn new(f: F) -> Self {
    Self {
      transformer: MapTransformer::new(f),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<I>) -> Self {
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

impl<F, I, O> Clone for Map<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<F, I, O> Input for Map<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = I;
  type InputStream = Pin<Box<dyn Stream<Item = I> + Send>>;
}

impl<F, I, O> Output for Map<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = O;
  type OutputStream = Pin<Box<dyn Stream<Item = O> + Send>>;
}

#[async_trait]
impl<F, I, O> Transformer for Map<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (I,);
  type OutputPorts = (O,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<I>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<I> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<I> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<I>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<I>) -> ErrorContext<I> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}

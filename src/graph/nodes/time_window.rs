//! Time window node for StreamWeave graphs
//!
//! Creates time-based windows of items. Groups consecutive items into windows
//! based on time duration, producing vectors of items as windows are created.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::TimeWindowTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::time::Duration;

/// Node that creates time-based windows of items.
///
/// This node wraps `TimeWindowTransformer` for use in graphs. It groups
/// consecutive items into windows based on time duration, producing vectors
/// of items as windows are created.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{TimeWindow, TransformerNode};
/// use tokio::time::Duration;
///
/// let time_window = TimeWindow::new(Duration::from_secs(5));
/// let node = TransformerNode::from_transformer(
///     "time_window".to_string(),
///     time_window,
/// );
/// ```
pub struct TimeWindow<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying time window transformer
  transformer: TimeWindowTransformer<T>,
}

impl<T> TimeWindow<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `TimeWindow` node with the specified window duration.
  ///
  /// # Arguments
  ///
  /// * `duration` - The duration of each window.
  pub fn new(duration: Duration) -> Self {
    Self {
      transformer: TimeWindowTransformer::new(duration),
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
}

impl<T> Clone for TimeWindow<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for TimeWindow<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for TimeWindow<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

#[async_trait]
impl<T> Transformer for TimeWindow<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (Vec<T>,);

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

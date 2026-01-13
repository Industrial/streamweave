//! Tokio channel send node for StreamWeave graphs
//!
//! Sends data to a tokio channel while passing data through. Takes data as input,
//! sends to channel, and outputs the same data, enabling sending to channel and continuing processing.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::TokioChannelSendTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::sync::mpsc::Sender;

/// Node that sends data to a tokio channel while passing data through.
///
/// This node wraps `TokioChannelSendTransformer` for use in graphs. It takes data as input,
/// sends it to a tokio channel, and outputs the same data, enabling sending to channel and continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{TokioChannelSend, TransformerNode};
/// use tokio::sync::mpsc;
///
/// let (tx, _rx) = mpsc::channel(100);
/// let tokio_channel_send = TokioChannelSend::new(tx);
/// let node = TransformerNode::from_transformer(
///     "tokio_channel_send".to_string(),
///     tokio_channel_send,
/// );
/// ```
pub struct TokioChannelSend<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying tokio channel send transformer
  transformer: TokioChannelSendTransformer<T>,
}

impl<T> TokioChannelSend<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `TokioChannelSend` node with the given sender.
  ///
  /// # Arguments
  ///
  /// * `sender` - The `tokio::sync::mpsc::Sender` to send items to.
  pub fn new(sender: Sender<T>) -> Self {
    Self {
      transformer: TokioChannelSendTransformer::new(sender),
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

impl<T> Clone for TokioChannelSend<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for TokioChannelSend<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for TokioChannelSend<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for TokioChannelSend<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
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

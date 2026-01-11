//! # Tokio Channel Send Transformer
//!
//! Transformer that sends data to a `tokio::sync::mpsc::Sender` while passing the
//! same data through to the output stream. This enables sending items to a channel
//! for parallel processing while continuing the main pipeline flow.
//!
//! ## Overview
//!
//! The Tokio Channel Send Transformer provides:
//!
//! - **Channel Integration**: Sends items to a tokio mpsc channel
//! - **Pass-Through**: Outputs the same items that were sent to the channel
//! - **Non-Blocking**: Channel sends are non-blocking (drops items if channel is full)
//! - **Error Handling**: Configurable error strategies for send failures
//! - **Type Safety**: Generic over any `Send + Sync + Clone` type
//!
//! ## Input/Output
//!
//! - **Input**: `Message<T>` - Items to send to the channel
//! - **Output**: `Message<T>` - The same items (pass-through)
//!
//! ## Use Cases
//!
//! - **Parallel Processing**: Send items to a separate processing pipeline
//! - **Monitoring**: Send items to a monitoring/logging channel
//! - **Fan-Out**: Distribute items to multiple consumers
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::transformers::TokioChannelSendTransformer;
//! use tokio::sync::mpsc;
//!
//! let (tx, _rx) = mpsc::channel(100);
//! let transformer = TokioChannelSendTransformer::new(tx);
//! // Input: ["hello", "world"]
//! // Sends to channel and outputs: ["hello", "world"]
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc::Sender};
use tracing::{error, warn};

/// A transformer that sends data to a tokio channel while passing data through.
///
/// Each input item is sent to a `tokio::sync::mpsc::Sender`,
/// and then the same item is output, enabling sending to channel and continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::TokioChannelSendTransformer;
/// use tokio::sync::mpsc;
///
/// let (tx, _rx) = mpsc::channel(100);
/// let transformer = TokioChannelSendTransformer::new(tx);
/// // Input: ["hello", "world"]
/// // Sends to channel and outputs: ["hello", "world"]
/// ```
pub struct TokioChannelSendTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The channel sender to forward items to (shared across stream items).
  channel: Arc<Mutex<Option<Sender<T>>>>,
  /// Transformer configuration.
  config: TransformerConfig<T>,
}

impl<T> TokioChannelSendTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `TokioChannelSendTransformer` with the given sender.
  ///
  /// # Arguments
  ///
  /// * `sender` - The `tokio::sync::mpsc::Sender` to send items to.
  #[must_use]
  pub fn new(sender: Sender<T>) -> Self {
    Self {
      channel: Arc::new(Mutex::new(Some(sender))),
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Clone for TokioChannelSendTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      channel: Arc::clone(&self.channel),
      config: self.config.clone(),
    }
  }
}

impl<T> Input for TokioChannelSendTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for TokioChannelSendTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for TokioChannelSendTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "tokio_channel_send_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let channel = Arc::clone(&self.channel);

    Box::pin(async_stream::stream! {
      let mut input_stream = input;
      while let Some(item) = input_stream.next().await {
        // Send to channel
        {
          let channel_guard = channel.lock().await;
          if let Some(ref sender) = *channel_guard {
            match sender.send(item.clone()).await {
              Ok(_) => {
                // Successfully sent
              }
              Err(e) => {
                let stream_error = StreamError::new(
                  Box::new(e),
                  ErrorContext {
                    timestamp: chrono::Utc::now(),
                    item: Some(item.clone()),
                    component_name: component_name.clone(),
                    component_type: std::any::type_name::<TokioChannelSendTransformer<T>>().to_string(),
                  },
                  ComponentInfo {
                    name: component_name.clone(),
                    type_name: std::any::type_name::<TokioChannelSendTransformer<T>>().to_string(),
                  },
                );

                match handle_error_strategy(&error_strategy, &stream_error) {
                  ErrorAction::Stop => {
                    error!(
                      component = %component_name,
                      error = %stream_error,
                      "Stopping due to channel send error"
                    );
                    return;
                  }
                  ErrorAction::Skip => {
                    warn!(
                      component = %component_name,
                      error = %stream_error,
                      "Skipping item due to channel send error"
                    );
                    continue;
                  }
                  ErrorAction::Retry => {
                    warn!(
                      component = %component_name,
                      error = %stream_error,
                      "Retry not supported for channel send errors, skipping"
                    );
                    continue;
                  }
                }
              }
            }
          }
        }

        // Pass through the item
        yield item;
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "tokio_channel_send_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "tokio_channel_send_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

/// Helper function to handle error strategy
pub(crate) fn handle_error_strategy<T>(
  strategy: &ErrorStrategy<T>,
  error: &StreamError<T>,
) -> ErrorAction
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
  match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    ErrorStrategy::Skip => ErrorAction::Skip,
    ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}

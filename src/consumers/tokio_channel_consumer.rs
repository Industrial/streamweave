//! Tokio channel consumer for forwarding stream data to Tokio channels.
//!
//! This module provides [`TokioChannelConsumer`], a consumer that forwards stream
//! items to a `tokio::sync::mpsc::Sender`. This allows StreamWeave pipelines to
//! integrate with existing async code that uses Tokio channels for communication.
//!
//! # Overview
//!
//! [`TokioChannelConsumer`] is useful for integrating StreamWeave pipelines with
//! existing async codebases that use Tokio channels. It forwards all stream items
//! to the provided channel sender, allowing StreamWeave to interoperate with other
//! async components.
//!
//! # Key Concepts
//!
//! - **Channel Integration**: Forwards items to Tokio's mpsc channel sender
//! - **Async Communication**: Uses async channel sends for non-blocking operation
//! - **Type Preservation**: Preserves the item type through the channel
//! - **Error Handling**: Configurable error strategies for channel send failures
//!
//! # Core Types
//!
//! - **[`TokioChannelConsumer<T>`]**: Consumer that forwards items to a Tokio channel
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::TokioChannelConsumer;
//! use futures::stream;
//! use tokio::sync::mpsc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a Tokio channel
//! let (sender, mut receiver) = mpsc::channel(100);
//!
//! // Create a consumer with the sender
//! let mut consumer = TokioChannelConsumer::new(sender);
//!
//! // Create a stream of items
//! let stream = stream::iter(vec![1, 2, 3, 4, 5]);
//!
//! // Consume the stream (items forwarded to channel)
//! consumer.consume(Box::pin(stream)).await;
//!
//! // Items are now available in the receiver
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::consumers::TokioChannelConsumer;
//! use streamweave::ErrorStrategy;
//! use tokio::sync::mpsc;
//!
//! // Create a channel
//! let (sender, _receiver) = mpsc::channel(10);
//!
//! // Create a consumer with error handling
//! let consumer = TokioChannelConsumer::new(sender)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("channel-forwarder".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Tokio Integration**: Uses Tokio's standard mpsc channels for compatibility
//!   with existing async code
//! - **Channel Ownership**: Takes ownership of the sender, allowing flexible usage
//! - **Async Sends**: Uses async channel sends to handle backpressure gracefully
//! - **Type Preservation**: Preserves item types through the channel
//!
//! # Integration with StreamWeave
//!
//! [`TokioChannelConsumer`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ConsumerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::sync::mpsc::Sender;

/// A consumer that sends items to a `tokio::sync::mpsc::Sender`.
///
/// This consumer forwards all items from the stream to the channel sender.
/// It's useful for integrating StreamWeave with existing async code that uses channels.
pub struct TokioChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The channel sender to forward items to.
  pub channel: Option<Sender<T>>,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

impl<T> TokioChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `TokioChannelConsumer` with the given sender.
  ///
  /// # Arguments
  ///
  /// * `sender` - The `tokio::sync::mpsc::Sender` to send items to.
  pub fn new(sender: Sender<T>) -> Self {
    Self {
      channel: Some(sender),
      config: ConsumerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this consumer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this consumer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this consumer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}

impl<T> Input for TokioChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Consumer for TokioChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, input: Self::InputStream) -> () {
    let mut stream = input;
    while let Some(value) = stream.next().await {
      if let Some(sender) = &self.channel
        && let Err(e) = sender.send(value).await
      {
        eprintln!("Failed to send value to channel: {}", e);
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: if self.config.name.is_empty() {
        "tokio_channel_consumer".to_string()
      } else {
        self.config.name.clone()
      },
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

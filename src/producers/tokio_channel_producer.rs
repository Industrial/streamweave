//! Tokio channel producer for reading stream data from Tokio channels.
//!
//! This module provides [`TokioChannelProducer`], a producer that reads items from
//! a `tokio::sync::mpsc::Receiver` and converts them into a StreamWeave stream.
//! It's useful for integrating StreamWeave with existing async Rust code that uses
//! Tokio channels.
//!
//! # Overview
//!
//! [`TokioChannelProducer`] bridges Tokio's channel system with StreamWeave's streaming
//! model. It takes ownership of a `Receiver` and produces items from the channel as
//! a stream, making it easy to integrate StreamWeave into existing Tokio-based applications.
//!
//! # Key Concepts
//!
//! - **Channel Integration**: Reads from `tokio::sync::mpsc::Receiver`
//! - **Stream Conversion**: Converts channel receiver into a StreamWeave stream
//! - **Ownership Transfer**: Takes ownership of the receiver (single-use)
//! - **Async Compatibility**: Fully compatible with Tokio's async runtime
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`TokioChannelProducer<T>`]**: Producer that reads from a Tokio channel receiver
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::producers::TokioChannelProducer;
//! use streamweave::PipelineBuilder;
//! use tokio::sync::mpsc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a Tokio channel
//! let (tx, rx) = mpsc::channel(100);
//!
//! // Create a producer from the receiver
//! let producer = TokioChannelProducer::new(rx);
//!
//! // Use in a pipeline
//! let pipeline = PipelineBuilder::new()
//!     .producer(producer)
//!     .transformer(/* ... */)
//!     .consumer(/* ... */);
//!
//! // Send items to the channel
//! tx.send("item1".to_string()).await?;
//! tx.send("item2".to_string()).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::producers::TokioChannelProducer;
//! use streamweave::ErrorStrategy;
//! use tokio::sync::mpsc;
//!
//! let (_, rx) = mpsc::channel(100);
//! // Create a producer with error handling strategy
//! let producer = TokioChannelProducer::new(rx)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("channel-reader".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Ownership Transfer**: Takes ownership of the receiver to ensure single-use
//!   semantics and prevent multiple consumers from the same channel
//! - **Stream Conversion**: Uses `tokio_stream::ReceiverStream` to convert the
//!   channel receiver into a standard async stream
//! - **Generic Type**: Generic over the item type for maximum flexibility
//! - **Tokio Compatibility**: Designed specifically for Tokio channels to leverage
//!   Tokio's efficient async I/O
//!
//! # Integration with StreamWeave
//!
//! [`TokioChannelProducer`] implements the [`Producer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ProducerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use futures::Stream;
use std::pin::Pin;
use tokio::sync::mpsc::Receiver;
use tokio_stream::wrappers::ReceiverStream;

/// A producer that reads items from a `tokio::sync::mpsc::Receiver`.
///
/// This producer reads items from a tokio channel receiver and produces them
/// into a StreamWeave stream. It's useful for integrating StreamWeave with
/// existing async code that uses channels.
pub struct TokioChannelProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The channel receiver to read items from.
  pub receiver: Option<Receiver<T>>,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<T>,
}

impl<T> TokioChannelProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `TokioChannelProducer` with the given receiver.
  ///
  /// # Arguments
  ///
  /// * `receiver` - The `tokio::sync::mpsc::Receiver` to read items from.
  pub fn new(receiver: Receiver<T>) -> Self {
    Self {
      receiver: Some(receiver),
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Output for TokioChannelProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Producer for TokioChannelProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type OutputPorts = (T,);

  fn produce(&mut self) -> Self::OutputStream {
    // Take ownership of the receiver
    let receiver = self.receiver.take().expect("Receiver already consumed");

    // Convert tokio Receiver to a Stream
    let stream = ReceiverStream::new(receiver);

    Box::pin(stream)
  }

  fn set_config_impl(&mut self, config: ProducerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<T> {
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
        .unwrap_or_else(|| "tokio_channel_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "tokio_channel_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

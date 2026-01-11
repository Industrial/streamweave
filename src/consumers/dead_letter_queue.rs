//! Dead letter queue consumer for collecting failed stream items.
//!
//! This module provides [`DeadLetterQueue`] and [`DeadLetterItem`], a consumer
//! that collects failed items from streams into an in-memory queue. It's designed
//! to work with error routing (e.g., `ErrorBranch`) to capture items that fail
//! processing for later analysis, retry, or manual intervention.
//!
//! # Overview
//!
//! [`DeadLetterQueue`] is useful for error handling patterns where failed items
//! need to be captured and processed separately. It stores items with timestamps
//! for audit purposes and provides methods to inspect, drain, and retrieve the
//! collected items.
//!
//! # Key Concepts
//!
//! - **Error Collection**: Collects items that have failed processing
//! - **Metadata Preservation**: Stores timestamps with each item for audit trails
//! - **In-Memory Storage**: Uses an in-memory queue for fast access
//! - **Thread-Safe**: Uses `Arc<Mutex<>>` for concurrent access
//!
//! # Core Types
//!
//! - **[`DeadLetterQueue<T>`]**: Consumer that collects failed items into a queue
//! - **[`DeadLetterItem<T>`]**: An item in the dead letter queue with metadata
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::DeadLetterQueue;
//! use futures::stream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a dead letter queue
//! let mut dlq = DeadLetterQueue::<i32>::new();
//!
//! // Create a stream of failed items
//! let error_stream = stream::iter(vec![1, 2, 3]);
//!
//! // Consume the stream (items collected in DLQ)
//! dlq.consume(Box::pin(error_stream)).await;
//!
//! // Check queue size
//! let count = dlq.len().await;
//! assert_eq!(count, 3);
//!
//! // Retrieve all items
//! let items = dlq.drain().await;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Pre-allocated Capacity
//!
//! ```rust
//! use streamweave::consumers::DeadLetterQueue;
//!
//! // Create a DLQ with pre-allocated capacity
//! let dlq = DeadLetterQueue::<String>::with_capacity(1000)
//!     .with_name("error-collector".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **In-Memory Storage**: Uses in-memory storage for fast access and simple
//!   implementation
//! - **Timestamp Metadata**: Stores timestamps to enable time-based analysis and
//!   expiration policies
//! - **Thread-Safe**: Uses `Arc<Mutex<>>` to support concurrent access patterns
//! - **Error Branch Integration**: Designed to work with error routing for
//!   automatic error collection
//!
//! # Integration with StreamWeave
//!
//! [`DeadLetterQueue`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It's particularly useful with error branching routers
//! to automatically capture failed items. It supports the standard error handling
//! strategies and configuration options provided by [`ConsumerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use chrono;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A consumer that collects failed items into a dead letter queue.
///
/// This consumer is designed to work with `ErrorBranch` router to collect
/// error items (failed IPs) for later analysis, retry, or manual processing.
/// It stores error items in memory with metadata about when they were received.
///
/// # Example
///
/// ```rust
/// use crate::Consumer;
/// use crate::consumers::DeadLetterQueue;
/// use futures::stream;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut dlq = DeadLetterQueue::<i32>::new();
/// let error_stream = Box::pin(stream::iter(vec![1, 2, 3]));
/// dlq.consume(error_stream).await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct DeadLetterQueue<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The internal queue where failed items are stored.
  pub queue: Arc<Mutex<Vec<DeadLetterItem<T>>>>,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

/// An item in the dead letter queue with metadata.
#[derive(Debug, Clone)]
pub struct DeadLetterItem<T> {
  /// The failed item.
  pub item: T,
  /// Timestamp when the item was added to the queue.
  pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> Default for DeadLetterQueue<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> DeadLetterQueue<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `DeadLetterQueue` with an empty queue.
  pub fn new() -> Self {
    Self {
      queue: Arc::new(Mutex::new(Vec::new())),
      config: ConsumerConfig::default(),
    }
  }

  /// Creates a new `DeadLetterQueue` with a pre-allocated queue capacity.
  ///
  /// # Arguments
  ///
  /// * `capacity` - The initial capacity of the internal queue.
  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      queue: Arc::new(Mutex::new(Vec::with_capacity(capacity))),
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

  /// Returns the current number of items in the dead letter queue.
  ///
  /// # Returns
  ///
  /// The number of items currently in the queue.
  pub async fn len(&self) -> usize {
    self.queue.lock().await.len()
  }

  /// Returns `true` if the dead letter queue is empty.
  ///
  /// # Returns
  ///
  /// `true` if the queue is empty, `false` otherwise.
  pub async fn is_empty(&self) -> bool {
    self.queue.lock().await.is_empty()
  }

  /// Retrieves all items from the dead letter queue and clears it.
  ///
  /// # Returns
  ///
  /// A vector of all dead letter items that were in the queue.
  pub async fn drain(&self) -> Vec<DeadLetterItem<T>> {
    let mut queue = self.queue.lock().await;
    std::mem::take(&mut *queue)
  }

  /// Retrieves all items from the dead letter queue without clearing it.
  ///
  /// # Returns
  ///
  /// A clone of all dead letter items currently in the queue.
  pub async fn items(&self) -> Vec<DeadLetterItem<T>> {
    self.queue.lock().await.clone()
  }

  /// Consumes the consumer and returns the collected queue.
  ///
  /// # Returns
  ///
  /// The vector containing all dead letter items in order.
  pub async fn into_queue(self) -> Vec<DeadLetterItem<T>> {
    Arc::try_unwrap(self.queue)
      .expect("DeadLetterQueue has multiple references")
      .into_inner()
  }
}

impl<T> Input for DeadLetterQueue<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Consumer for DeadLetterQueue<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let consumer_name = self.config.name.clone();
    let queue = Arc::clone(&self.queue);
    let mut count = 0;

    while let Some(item) = stream.next().await {
      count += 1;
      let timestamp = chrono::Utc::now();
      let dead_letter_item = DeadLetterItem { item, timestamp };

      queue.lock().await.push(dead_letter_item);
    }

    if !consumer_name.is_empty() {
      println!(
        "📬 [{}] Dead letter queue received {} items",
        consumer_name, count
      );
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
      component_name: self.config.name.clone(),
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

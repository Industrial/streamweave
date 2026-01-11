//! Vector consumer for collecting stream items into a Vec.
//!
//! This module provides [`VecConsumer`], a consumer that collects all stream items
//! into an in-memory `Vec`. It's useful for testing, debugging, and scenarios where
//! all items need to be collected before processing.
//!
//! # Overview
//!
//! [`VecConsumer`] is useful for collecting stream items into a vector for batch
//! processing, testing, and debugging. It stores all items in memory in the order
//! they are received, and provides methods to access and consume the collected items.
//!
//! # Key Concepts
//!
//! - **In-Memory Collection**: Collects all items into a `Vec` in memory
//! - **Order Preservation**: Items are stored in the order they are received
//! - **Consumption**: Provides methods to extract the collected vector
//! - **Capacity Management**: Supports pre-allocated capacity for performance
//!
//! # Core Types
//!
//! - **[`VecConsumer<T>`]**: Consumer that collects items into a Vec
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::VecConsumer;
//! use futures::stream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a consumer
//! let mut consumer = VecConsumer::<i32>::new();
//!
//! // Create a stream of items
//! let stream = stream::iter(vec![1, 2, 3, 4, 5]);
//!
//! // Consume the stream (items collected in Vec)
//! consumer.consume(Box::pin(stream)).await;
//!
//! // Access collected items
//! assert_eq!(consumer.vec.len(), 5);
//!
//! // Extract the vector
//! let items = consumer.into_vec();
//! assert_eq!(items, vec![1, 2, 3, 4, 5]);
//! # Ok(())
//! # }
//! ```
//!
//! ## With Pre-allocated Capacity
//!
//! ```rust
//! use streamweave::consumers::VecConsumer;
//!
//! // Create a consumer with pre-allocated capacity
//! let consumer = VecConsumer::<String>::with_capacity(1000)
//!     .with_name("item-collector".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **In-Memory Storage**: Uses in-memory storage for simple, fast access
//! - **Order Preservation**: Maintains item order for predictable behavior
//! - **Capacity Management**: Supports pre-allocation to avoid reallocations
//! - **Simple Interface**: Provides straightforward methods for accessing collected items
//!
//! # Integration with StreamWeave
//!
//! [`VecConsumer`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ConsumerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A consumer that collects stream items into a Vec.
///
/// This consumer collects all items from the stream into a Vec.
/// Items are stored in the order they are received.
pub struct VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The Vec storing consumed items.
  pub vec: Vec<T>,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

impl<T> Default for VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `VecConsumer`.
  pub fn new() -> Self {
    Self {
      vec: Vec::new(),
      config: ConsumerConfig::default(),
    }
  }

  /// Creates a new `VecConsumer` with the specified capacity.
  ///
  /// # Arguments
  ///
  /// * `capacity` - The initial capacity of the Vec.
  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      vec: Vec::with_capacity(capacity),
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

  /// Consumes the consumer and returns the collected Vec.
  ///
  /// # Returns
  ///
  /// The Vec containing all consumed items.
  pub fn into_vec(self) -> Vec<T> {
    self.vec
  }
}

// Trait implementations for VecConsumer

impl<T> Input for VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

#[async_trait]
impl<T> Consumer for VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) {
    while let Some(item) = stream.next().await {
      self.vec.push(item);
    }
  }

  fn get_config_impl(&self) -> &ConsumerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<Self::Input> {
    &mut self.config
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<Self::Input>) {
    self.config = config;
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: "VecConsumer".to_string(),
    }
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Retry(_) => ErrorAction::Stop,
      ErrorStrategy::Custom(f) => f(error),
    }
  }
}

//! Array consumer for collecting stream items into fixed-size arrays.
//!
//! This module provides [`ArrayConsumer`], a consumer that collects stream items
//! into a fixed-size array. This is useful for scenarios where you need to collect
//! a known number of items from a stream into a statically-sized array.
//!
//! # Overview
//!
//! [`ArrayConsumer`] is useful for collecting a fixed number of items from a stream
//! into a statically-sized array. Unlike `VecConsumer`, which uses dynamic allocation,
//! `ArrayConsumer` uses a compile-time fixed-size array, making it suitable for
//! scenarios where the number of items is known at compile time.
//!
//! # Key Concepts
//!
//! - **Fixed-Size Collection**: Collects items into a compile-time fixed-size array
//! - **Static Allocation**: Uses stack-allocated arrays (for small sizes) or static arrays
//! - **Index Tracking**: Maintains an index to track the current position in the array
//! - **Generic Type**: Works with any item type that implements required traits
//!
//! # Core Types
//!
//! - **[`ArrayConsumer<T, N>`]**: Consumer that collects items into a fixed-size array of size `N`
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::ArrayConsumer;
//!
//! // Create an array consumer that collects 5 items
//! let consumer = ArrayConsumer::<i32, 5>::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::consumers::ArrayConsumer;
//! use streamweave::ErrorStrategy;
//!
//! // Create an array consumer with error handling strategy
//! let consumer = ArrayConsumer::<String, 10>::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("array-collector".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Fixed Size**: Uses compile-time constant `N` for array size, enabling static allocation
//! - **Option Wrapper**: Uses `Option<T>` for array elements to track which positions are filled
//! - **Index Tracking**: Maintains an index to know where to insert the next item
//! - **Generic Type**: Works with any item type `T` that implements required traits
//!
//! # Integration with StreamWeave
//!
//! [`ArrayConsumer`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`ConsumerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A consumer that collects stream items into a fixed-size array.
///
/// This consumer collects up to `N` items from the stream into a fixed-size array.
/// Items are stored in the order they are received.
pub struct ArrayConsumer<T, const N: usize>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The array storing consumed items.
  pub array: [Option<T>; N],
  /// The current index for inserting items.
  pub index: usize,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

impl<T, const N: usize> Default for ArrayConsumer<T, N>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T, const N: usize> ArrayConsumer<T, N>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `ArrayConsumer`.
  pub fn new() -> Self {
    Self {
      array: std::array::from_fn(|_| None),
      index: 0,
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

  /// Consumes the consumer and returns the collected array.
  ///
  /// # Returns
  ///
  /// The array containing all consumed items.
  pub fn into_array(self) -> [Option<T>; N] {
    self.array
  }
}

// Trait implementations for ArrayConsumer

impl<T, const N: usize> Input for ArrayConsumer<T, N>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T, const N: usize> Consumer for ArrayConsumer<T, N>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some(value) = stream.next().await {
      if self.index < N {
        self.array[self.index] = Some(value);
        self.index += 1;
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
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
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

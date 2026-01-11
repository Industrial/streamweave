//! Vector producer for producing stream data from a Vec.
//!
//! This module provides [`VecProducer<T>`], a producer that yields items from a `Vec`
//! in order. It's useful for converting in-memory collections into streams and for
//! testing and development.
//!
//! # Overview
//!
//! [`VecProducer`] takes a `Vec<T>` and produces each item as a stream element.
//! The items are produced in the order they appear in the vector, making it ideal
//! for testing, prototyping, and converting existing data structures into streams.
//!
//! # Key Concepts
//!
//! - **In-Memory Data**: Produces items from an in-memory `Vec`
//! - **Ordered Output**: Items are produced in the order they appear in the vector
//! - **Generic Type**: Generic over the item type for maximum flexibility
//! - **Cloning**: Clones the vector to avoid taking ownership (producer can be reused)
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`VecProducer<T>`]**: Producer that yields items from a `Vec<T>`
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::producers::VecProducer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer from a vector
//! let producer = VecProducer::new(vec![1, 2, 3, 4, 5]);
//!
//! // Use in a pipeline
//! let pipeline = PipelineBuilder::new()
//!     .producer(producer)
//!     .transformer(/* ... */)
//!     .consumer(/* ... */);
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::producers::VecProducer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a producer with error handling strategy
//! let producer = VecProducer::new(vec!["item1", "item2", "item3"])
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("vec-reader".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Cloning**: Clones the vector data to allow the producer to be used multiple
//!   times or stored without consuming the original data
//! - **Generic Type**: Generic over `T` to support any type that can be cloned
//! - **Simple Implementation**: Uses `stream::iter` for straightforward conversion
//!   from iterator to stream
//! - **Testing Tool**: Ideal for testing pipelines with known data
//!
//! # Integration with StreamWeave
//!
//! [`VecProducer`] implements the [`Producer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ProducerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use futures::{Stream, stream};
use std::pin::Pin;

/// A producer that yields items from a Vec.
///
/// This producer iterates over all items in the Vec and produces them
/// in order.
#[derive(Clone)]
pub struct VecProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The Vec data to produce from.
  pub data: Vec<T>,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> VecProducer<T> {
  /// Creates a new `VecProducer` with the given Vec.
  ///
  /// # Arguments
  ///
  /// * `data` - The Vec to produce items from.
  pub fn new(data: Vec<T>) -> Self {
    Self {
      data,
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

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for VecProducer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Producer for VecProducer<T> {
  type OutputPorts = (T,);

  fn produce(&mut self) -> Self::OutputStream {
    let stream = stream::iter(self.data.clone());
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
        .unwrap_or_else(|| "vec_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "vec_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

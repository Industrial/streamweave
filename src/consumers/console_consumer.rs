//! Console consumer for printing stream data to the console.
//!
//! This module provides [`ConsoleConsumer`], a consumer that prints stream items
//! to the console using `println!`. Each item is printed on a separate line, making
//! it ideal for debugging, testing, and simple command-line output.
//!
//! # Overview
//!
//! [`ConsoleConsumer`] is useful for debugging and testing StreamWeave pipelines.
//! It prints each stream item to stdout with automatic formatting, providing a
//! simple way to inspect stream contents during development.
//!
//! # Key Concepts
//!
//! - **Console Output**: Prints items to stdout using `println!`
//! - **Line-Based Output**: Each item is printed on a separate line
//! - **Display Trait**: Items must implement `Display` for string conversion
//! - **Debugging Tool**: Primarily intended for development and testing
//!
//! # Core Types
//!
//! - **[`ConsoleConsumer<T>`]**: Consumer that prints items to the console
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::ConsoleConsumer;
//! use futures::stream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a consumer
//! let mut consumer = ConsoleConsumer::new();
//!
//! // Create a stream of items
//! let stream = stream::iter(vec!["item1", "item2", "item3"]);
//!
//! // Consume the stream (items printed to console)
//! consumer.consume(Box::pin(stream)).await;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::consumers::ConsoleConsumer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a consumer with error handling strategy
//! let consumer = ConsoleConsumer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("console-debug".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Simple Output**: Uses `println!` for straightforward console output
//! - **Line Formatting**: Each item is printed on a separate line for readability
//! - **Display Requirement**: Items must implement `Display` for flexible formatting
//! - **Development Tool**: Designed primarily for debugging and testing
//!
//! # Integration with StreamWeave
//!
//! [`ConsoleConsumer`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ConsumerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::StreamExt;

/// A consumer that prints items to the console.
///
/// This consumer prints each item from the stream to stdout using `println!`.
/// It's useful for debugging, testing, and simple command-line output.
///
/// # Example
///
/// ```rust,no_run
/// use crate::consumers::ConsoleConsumer;
/// use crate::PipelineBuilder;
///
/// let consumer = ConsoleConsumer::new();
/// let pipeline = PipelineBuilder::new()
///     .producer(/* ... */)
///     .transformer(/* ... */)
///     .consumer(consumer);
/// ```
pub struct ConsoleConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

impl<T> ConsoleConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  /// Creates a new `ConsoleConsumer` with default configuration.
  pub fn new() -> Self {
    Self {
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

impl<T> Default for ConsoleConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

// Trait implementations for ConsoleConsumer

impl<T> Input for ConsoleConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  type Input = T;
  type InputStream = futures::stream::BoxStream<'static, T>;
}

#[async_trait]
impl<T> Consumer for ConsoleConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some(value) = stream.next().await {
      println!("{}", value);
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
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: if self.config.name.is_empty() {
        "console_consumer".to_string()
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

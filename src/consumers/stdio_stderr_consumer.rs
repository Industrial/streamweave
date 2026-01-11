//! Standard error consumer for writing stream data to stderr.
//!
//! This module provides [`StdioStderrConsumer`], a consumer that writes stream items
//! to standard error (stderr). Each item is written on a separate line, making it
//! ideal for error output, diagnostics, and logging in command-line tools.
//!
//! # Overview
//!
//! [`StdioStderrConsumer`] is useful for building command-line tools that need to
//! output errors or diagnostics to stderr while keeping stdout available for primary
//! output. It uses Tokio's async I/O for efficient writing and supports any type that
//! implements `Display`.
//!
//! # Key Concepts
//!
//! - **Standard Error Output**: Writes to stderr, separate from stdout
//! - **Line-Based Output**: Each item is written on a separate line
//! - **Display Trait**: Items must implement `Display` for string conversion
//! - **Async I/O**: Uses Tokio's async stderr handle for non-blocking writes
//!
//! # Core Types
//!
//! - **[`StdioStderrConsumer<T>`]**: Consumer that writes items to stderr
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::StdioStderrConsumer;
//! use futures::stream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a consumer
//! let mut consumer = StdioStderrConsumer::new();
//!
//! // Create a stream of error messages
//! let stream = stream::iter(vec!["Warning: low memory", "Error: connection failed"]);
//!
//! // Consume the stream (items written to stderr)
//! consumer.consume(Box::pin(stream)).await;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::consumers::StdioStderrConsumer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a consumer with error handling strategy
//! let consumer = StdioStderrConsumer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("error-logger".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Stderr Separation**: Uses stderr to keep errors separate from primary output
//! - **Line Formatting**: Each item is formatted with a trailing newline for readability
//! - **Display Requirement**: Items must implement `Display` for flexible output formatting
//! - **Flush on Completion**: Final flush ensures all output is written before completion
//!
//! # Integration with StreamWeave
//!
//! [`StdioStderrConsumer`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ConsumerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;

/// A consumer that writes items to standard error (stderr).
///
/// This consumer writes each item from the stream to stderr, one per line.
/// It's useful for building command-line tools that output errors or diagnostics to stderr.
///
/// # Example
///
/// ```rust,no_run
/// use crate::consumers::StdioStderrConsumer;
/// use crate::Pipeline;
///
/// let consumer = StdioStderrConsumer::new();
/// let pipeline = Pipeline::new()
///     .producer(/* ... */)
///     .transformer(/* ... */)
///     .consumer(consumer);
/// ```
pub struct StdioStderrConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

impl<T> StdioStderrConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  /// Creates a new `StdioStderrConsumer` with default configuration.
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

impl<T> Default for StdioStderrConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

// Trait implementations for StdioStderrConsumer

impl<T> Input for StdioStderrConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  type Input = T;
  type InputStream = futures::stream::BoxStream<'static, T>;
}

#[async_trait]
impl<T> Consumer for StdioStderrConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let mut stderr = tokio::io::stderr();
    let component_name = self.config.name.clone();

    while let Some(value) = stream.next().await {
      let output = format!("{}\n", value);
      match stderr.write_all(output.as_bytes()).await {
        Ok(_) => {}
        Err(e) => {
          tracing::warn!(
            component = %component_name,
            error = %e,
            "Failed to write to stderr, continuing"
          );
        }
      }
    }

    // Flush stderr to ensure all output is written
    if let Err(e) = stderr.flush().await {
      tracing::warn!(
        component = %component_name,
        error = %e,
        "Failed to flush stderr"
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
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: if self.config.name.is_empty() {
        "stdio_stderr_consumer".to_string()
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

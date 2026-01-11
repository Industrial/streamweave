//! Standard input (stdin) producer for reading stream data from stdin.
//!
//! This module provides [`StdioStdinProducer`], a producer that reads lines from
//! standard input and emits each line as a string item. It's useful for building
//! command-line tools that process input from stdin.
//!
//! # Overview
//!
//! [`StdioStdinProducer`] reads data from stdin line by line, making it ideal for
//! command-line applications that need to process text input. It uses Tokio's async
//! I/O for efficient reading and supports configurable error handling.
//!
//! # Key Concepts
//!
//! - **Stdin Reading**: Reads from standard input (stdin)
//! - **Line-Based Reading**: Reads input line by line, emitting each line as a string
//! - **Async I/O**: Uses Tokio's async I/O for efficient, non-blocking reads
//! - **Error Handling**: Configurable error strategies for I/O failures
//! - **Command-Line Tool Support**: Ideal for CLI applications that process stdin
//!
//! # Core Types
//!
//! - **[`StdioStdinProducer`]**: Producer that reads lines from stdin
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::producers::StdioStdinProducer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that reads from stdin
//! let producer = StdioStdinProducer::new();
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
//! use streamweave::producers::StdioStdinProducer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a producer with error handling strategy
//! let producer = StdioStdinProducer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)  // Skip errors and continue
//!     .with_name("stdin-reader".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Line-Based**: Reads stdin line by line, which is the standard pattern for
//!   text processing in command-line tools
//! - **Async I/O**: Uses Tokio's async I/O for efficient, non-blocking reads
//! - **String Output**: Emits `String` items, one per line from stdin
//! - **EOF Handling**: Stops reading when EOF is reached (stdin closes)
//! - **Error Logging**: I/O errors are logged and cause the stream to stop
//!
//! # Integration with StreamWeave
//!
//! [`StdioStdinProducer`] implements the [`Producer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ProducerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::io::{AsyncBufReadExt, BufReader};

/// A producer that reads lines from standard input (stdin).
///
/// This producer reads lines from stdin and emits each line as a string item.
/// It's useful for building command-line tools that process input from stdin.
///
/// # Example
///
/// ```rust,no_run
/// use crate::producers::StdioStdinProducer;
/// use crate::Pipeline;
///
/// let producer = StdioStdinProducer::new();
/// let pipeline = Pipeline::new()
///     .producer(producer)
///     .transformer(/* ... */)
///     .consumer(/* ... */);
/// ```
pub struct StdioStdinProducer {
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<String>,
}

impl StdioStdinProducer {
  /// Creates a new `StdioStdinProducer` with default configuration.
  pub fn new() -> Self {
    Self {
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
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

impl Default for StdioStdinProducer {
  fn default() -> Self {
    Self::new()
  }
}

// Trait implementations for StdioStdinProducer

impl Output for StdioStdinProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Producer for StdioStdinProducer {
  type OutputPorts = (String,);

  fn produce(&mut self) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "stdio_stdin_producer".to_string());

    Box::pin(async_stream::stream! {
      let stdin_handle = tokio::io::stdin();
      let reader = BufReader::new(stdin_handle);
      let mut lines = reader.lines();

      loop {
        match lines.next_line().await {
          Ok(Some(line)) => yield line,
          Ok(None) => break, // EOF
          Err(e) => {
            tracing::warn!(
              component = %component_name,
              error = %e,
              "Failed to read line from stdin, stopping"
            );
            break;
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "stdin_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "stdin_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

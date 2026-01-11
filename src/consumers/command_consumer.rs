//! Command execution consumer for streaming data to external commands.
//!
//! This module provides [`CommandConsumer`], a consumer that executes external commands
//! for each item in a stream. Each item is converted to a string and passed as an argument
//! to the configured command.
//!
//! # Overview
//!
//! [`CommandConsumer`] is useful for integrating StreamWeave pipelines with external tools
//! and scripts. It takes each stream item, converts it to a string via the `Display` trait,
//! and executes the configured command with the item as an argument.
//!
//! # Key Concepts
//!
//! - **Command Execution**: Each stream item triggers a separate command execution
//! - **String Conversion**: Items must implement `Display` to be converted to command arguments
//! - **Error Handling**: Configurable error strategies (stop, skip, retry, custom)
//! - **Async Execution**: Uses Tokio's async command execution for non-blocking operations
//!
//! # Core Types
//!
//! - **[`CommandConsumer<T>`]**: Consumer that executes commands for each stream item
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::CommandConsumer;
//! use streamweave::{Consumer, ErrorStrategy};
//! use futures::stream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a consumer that executes 'echo' for each item
//! let mut consumer = CommandConsumer::new("echo".to_string(), vec![]);
//!
//! // Create a stream of items
//! let stream = stream::iter(vec!["hello", "world", "rust"]);
//!
//! // Consume the stream
//! consumer.consume(Box::pin(stream)).await;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::consumers::CommandConsumer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a consumer with custom error handling
//! let consumer = CommandConsumer::new("my_command".to_string(), vec![])
//!     .with_error_strategy(ErrorStrategy::Skip)  // Skip failed commands and continue
//!     .with_name("my-command-consumer".to_string());
//! ```
//!
//! ## With Command Arguments
//!
//! ```rust
//! use streamweave::consumers::CommandConsumer;
//!
//! // Execute 'grep' with pattern, where each item is the file to search
//! let consumer = CommandConsumer::new(
//!     "grep".to_string(),
//!     vec!["-r".to_string(), "pattern".to_string()]
//! );
//! ```
//!
//! # Design Decisions
//!
//! - **Display Trait Requirement**: Items must implement `Display` to enable flexible
//!   string conversion for command arguments
//! - **Per-Item Execution**: Each item triggers a separate command execution, allowing
//!   fine-grained control but potentially higher overhead
//! - **Async Execution**: Uses Tokio's async `Command` for non-blocking command execution
//!
//! # Integration with StreamWeave
//!
//! [`CommandConsumer`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ConsumerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::process::Command;

/// A consumer that executes an external command for each item.
///
/// This consumer takes each item from the stream, converts it to a string (via `Display`),
/// and executes the configured command with the item as input.
pub struct CommandConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  /// The command to execute for each item.
  pub command: Option<Command>,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

impl<T> CommandConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  /// Creates a new `CommandConsumer` with the given command and arguments.
  ///
  /// # Arguments
  ///
  /// * `command` - The command to execute.
  /// * `args` - Arguments to pass to the command.
  pub fn new(command: String, args: Vec<String>) -> Self {
    let mut cmd = Command::new(command);
    cmd.args(args);
    Self {
      command: Some(cmd),
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

// Trait implementations for CommandConsumer

impl<T> Input for CommandConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Consumer for CommandConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some(value) = stream.next().await {
      if let Some(cmd) = &mut self.command {
        let output = cmd.arg(value.to_string()).output().await;
        if let Err(e) = output {
          eprintln!("Failed to execute command: {}", e);
        }
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

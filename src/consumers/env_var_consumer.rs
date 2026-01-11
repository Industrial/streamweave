//! Environment variable consumer for setting environment variables from stream data.
//!
//! This module provides [`EnvVarConsumer`], a consumer that sets environment variables
//! from key-value pairs in a stream. Each stream item is a `(String, String)` tuple
//! representing an environment variable name and value.
//!
//! # Overview
//!
//! [`EnvVarConsumer`] is useful for dynamically configuring the runtime environment
//! based on stream data. It validates environment variable names according to standard
//! conventions and handles errors gracefully using configurable error strategies.
//!
//! # Key Concepts
//!
//! - **Key-Value Pairs**: Input must be `(String, String)` tuples representing variable
//!   name and value
//! - **Name Validation**: Environment variable names are validated before setting
//! - **Error Handling**: Configurable error strategies for invalid variable names
//! - **Thread Safety**: Environment variables are set using `std::env::set_var`
//!
//! # Core Types
//!
//! - **[`EnvVarConsumer`]**: Consumer that sets environment variables from stream items
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::EnvVarConsumer;
//! use futures::stream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a consumer
//! let mut consumer = EnvVarConsumer::new();
//!
//! // Create a stream of key-value pairs
//! let stream = stream::iter(vec![
//!     ("VAR1".to_string(), "value1".to_string()),
//!     ("VAR2".to_string(), "value2".to_string()),
//! ]);
//!
//! // Consume the stream to set environment variables
//! consumer.consume(Box::pin(stream)).await;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::consumers::EnvVarConsumer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a consumer with error handling strategy
//! let consumer = EnvVarConsumer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)  // Skip invalid names and continue
//!     .with_name("env-config".to_string());
//! ```
//!
//! # Environment Variable Name Validation
//!
//! Environment variable names are validated according to standard conventions:
//!
//! - Must not be empty
//! - Must start with a letter or underscore
//! - Can contain letters, digits, and underscores
//! - Cannot contain special characters or spaces
//!
//! Invalid names trigger the configured error handling strategy (stop, skip, or retry).
//!
//! # Design Decisions
//!
//! - **Validation**: Environment variable names are validated to prevent invalid
//!   assignments and provide clear error messages
//! - **Error Handling**: Invalid names trigger configurable error strategies rather
//!   than silently failing
//! - **Type Safety**: Requires `(String, String)` tuples to ensure correct key-value
//!   structure
//!
//! # Integration with StreamWeave
//!
//! [`EnvVarConsumer`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ConsumerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tracing::{error, warn};

/// A consumer that sets environment variables from key-value pairs.
///
/// This consumer takes `(String, String)` tuples from the stream and sets them
/// as environment variables. It's useful for configuring the runtime environment
/// based on stream data.
#[derive(Debug, Clone)]
pub struct EnvVarConsumer {
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<(String, String)>,
}

impl EnvVarConsumer {
  /// Creates a new `EnvVarConsumer`.
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
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<(String, String)>) -> Self {
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

  /// Validates if a string is a valid environment variable name.
  ///
  /// Environment variable names typically:
  /// - Must not be empty
  /// - Should start with a letter or underscore
  /// - Can contain letters, digits, and underscores
  /// - Should not contain special characters or spaces
  fn is_valid_env_var_name(name: &str) -> bool {
    if name.is_empty() {
      return false;
    }

    // First character must be a letter or underscore
    let first_char = name.chars().next().unwrap();
    if !first_char.is_alphabetic() && first_char != '_' {
      return false;
    }

    // All characters must be alphanumeric or underscore
    name.chars().all(|c| c.is_alphanumeric() || c == '_')
  }
}

impl Default for EnvVarConsumer {
  fn default() -> Self {
    Self::new()
  }
}

impl Input for EnvVarConsumer {
  type Input = (String, String);
  type InputStream = Pin<Box<dyn Stream<Item = (String, String)> + Send>>;
}

#[async_trait]
impl Consumer for EnvVarConsumer {
  type InputPorts = ((String, String),);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some((key, value)) = stream.next().await {
      // Validate the key before setting
      if Self::is_valid_env_var_name(&key) {
        unsafe {
          std::env::set_var(&key, &value);
        }
      } else {
        let error_msg = format!("Invalid environment variable name: {}", key);
        warn!("{}", error_msg);

        // Create error context for invalid variable names
        let error_context = self.create_error_context(Some((key.clone(), value)));
        let stream_error = StreamError {
          source: Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            error_msg,
          )),
          context: error_context,
          component: self.component_info(),
          retries: 0,
        };

        match self.handle_error(&stream_error) {
          ErrorAction::Stop => {
            error!("Stopping due to invalid environment variable name: {}", key);
            break;
          }
          ErrorAction::Skip => {
            warn!("Skipping invalid environment variable: {}", key);
            continue;
          }
          ErrorAction::Retry => {
            warn!("Retrying invalid environment variable: {}", key);
            // For retry, we'll just skip since we can't retry a validation error
            continue;
          }
        }
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<(String, String)>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<(String, String)> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<(String, String)> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<(String, String)>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<(String, String)>) -> ErrorContext<(String, String)> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: if self.config.name.is_empty() {
        "env_var_consumer".to_string()
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

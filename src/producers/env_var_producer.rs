//! # Environment Variable Producer
//!
//! Producer for reading environment variables in StreamWeave pipelines.
//!
//! This module provides [`EnvVarProducer`], a producer that reads environment variables
//! from the process environment and emits them as key-value pairs. It supports reading
//! all environment variables or filtering to specific variable names.
//!
//! # Overview
//!
//! [`EnvVarProducer`] is useful for configuration injection, environment-based processing,
//! and system integration scenarios. It reads environment variables at production time
//! and emits them as `(String, String)` tuples representing key-value pairs.
//!
//! # Key Concepts
//!
//! - **Environment Variables**: Reads from the process environment using `std::env::vars()`
//! - **Key-Value Pairs**: Emits environment variables as `(String, String)` tuples
//! - **Filtering**: Supports filtering to specific environment variable names
//! - **Synchronous Reading**: Reads environment variables synchronously (no async I/O needed)
//! - **Error Handling**: Configurable error strategies (though environment reading rarely fails)
//!
//! # Core Types
//!
//! - **[`EnvVarProducer`]**: Producer that reads and emits environment variables
//!
//! # Quick Start
//!
//! ## Basic Usage (All Environment Variables)
//!
//! ```rust
//! use streamweave::producers::EnvVarProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that reads all environment variables
//! let mut producer = EnvVarProducer::new();
//!
//! // Generate the stream
//! let mut stream = producer.produce();
//!
//! // Process environment variables
//! while let Some((key, value)) = stream.next().await {
//!     println!("{} = {}", key, value);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Filtered Environment Variables
//!
//! ```rust
//! use streamweave::producers::EnvVarProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that only reads specific environment variables
//! let mut producer = EnvVarProducer::with_vars(vec![
//!     "PATH".to_string(),
//!     "HOME".to_string(),
//!     "USER".to_string(),
//! ]);
//!
//! let mut stream = producer.produce();
//!
//! // Process only the specified environment variables
//! while let Some((key, value)) = stream.next().await {
//!     println!("{} = {}", key, value);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::producers::EnvVarProducer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a producer with error handling strategy
//! let producer = EnvVarProducer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("env-reader".to_string());
//! ```
//!
//! ## Configuration Injection Pattern
//!
//! ```rust,no_run
//! use streamweave::producers::EnvVarProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Read configuration from environment variables
//! let mut producer = EnvVarProducer::with_vars(vec![
//!     "DATABASE_URL".to_string(),
//!     "API_KEY".to_string(),
//!     "LOG_LEVEL".to_string(),
//! ]);
//!
//! let mut stream = producer.produce();
//! while let Some((key, value)) = stream.next().await {
//!     // Process configuration values
//!     match key.as_str() {
//!         "DATABASE_URL" => println!("Database: {}", value),
//!         "API_KEY" => println!("API key set"),
//!         "LOG_LEVEL" => println!("Log level: {}", value),
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Design Decisions
//!
//! ## Synchronous Reading
//!
//! Environment variables are read synchronously using `std::env::vars()` because
//! environment variable access is a fast, in-memory operation that doesn't require
//! async I/O. The stream itself is async, but the environment variable reading is
//! done synchronously.
//!
//! ## Key-Value Tuple Output
//!
//! Emits environment variables as `(String, String)` tuples for clarity and ease
//! of use. This format clearly represents the key-value nature of environment variables.
//!
//! ## Filtering Support
//!
//! Supports filtering to specific environment variable names to reduce noise and
//! improve performance when only a subset of variables is needed. Variables that
//! don't exist are simply not yielded (not an error).
//!
//! ## Missing Variables
//!
//! Environment variables that are requested but don't exist are simply not yielded
//! (silently skipped). This design prevents errors when variables are optional and
//! allows for flexible configuration patterns.
//!
//! # Integration with StreamWeave
//!
//! [`EnvVarProducer`] integrates seamlessly with StreamWeave's pipeline and graph systems:
//!
//! - **Pipeline API**: Use in pipelines for environment-based configuration
//! - **Graph API**: Wrap in graph nodes for graph-based environment processing
//! - **Error Handling**: Supports standard error handling strategies
//! - **Configuration**: Supports configuration via [`ProducerConfig`]
//! - **Transformers**: Environment variables can be transformed, filtered, or processed
//!
//! # Common Patterns
//!
//! ## Configuration Loading
//!
//! Load configuration from environment variables:
//!
//! ```rust,no_run
//! use streamweave::producers::EnvVarProducer;
//! use futures::StreamExt;
//! use std::collections::HashMap;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut producer = EnvVarProducer::new();
//! let mut stream = producer.produce();
//!
//! let mut config = HashMap::new();
//! while let Some((key, value)) = stream.next().await {
//!     config.insert(key, value);
//! }
//!
//! // Use config...
//! # Ok(())
//! # }
//! ```
//!
//! ## Environment-Based Routing
//!
//! Use environment variables to control pipeline behavior:
//!
//! ```rust,no_run
//! use streamweave::producers::EnvVarProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut producer = EnvVarProducer::with_vars(vec!["ENVIRONMENT".to_string()]);
//! let mut stream = producer.produce();
//!
//! if let Some((_, env)) = stream.next().await {
//!     match env.as_str() {
//!         "production" => println!("Using production config"),
//!         "development" => println!("Using development config"),
//!         _ => println!("Using default config"),
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Platform Notes
//!
//! - **Environment Access**: Uses `std::env::vars()` which is available on all platforms
//! - **Variable Names**: Variable names are platform-specific but typically case-sensitive on Unix
//! - **Value Encoding**: Values are read as UTF-8 strings (non-UTF-8 values may cause issues)
//! - **Process Environment**: Reads from the current process environment (not system-wide)

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use async_trait::async_trait;
use futures::{Stream, stream};
use std::pin::Pin;

/// A producer that yields environment variables as key-value pairs.
///
/// This producer reads environment variables and emits them as `(String, String)`
/// tuples. Optionally, it can filter to only specific variable names.
#[derive(Debug, Clone)]
pub struct EnvVarProducer {
  /// Optional filter to only include specific environment variable names.
  pub filter: Option<Vec<String>>,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<(String, String)>,
}

impl EnvVarProducer {
  /// Creates a new `EnvVarProducer` that produces all environment variables.
  pub fn new() -> Self {
    Self {
      filter: None,
      config: ProducerConfig::default(),
    }
  }

  /// Creates a new `EnvVarProducer` that only produces the specified environment variables.
  ///
  /// # Arguments
  ///
  /// * `vars` - The list of environment variable names to include.
  pub fn with_vars(vars: Vec<String>) -> Self {
    Self {
      filter: Some(vars),
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<(String, String)>) -> Self {
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

impl Default for EnvVarProducer {
  fn default() -> Self {
    Self::new()
  }
}

impl Output for EnvVarProducer {
  type Output = (String, String);
  type OutputStream = Pin<Box<dyn Stream<Item = (String, String)> + Send>>;
}

#[async_trait]
impl Producer for EnvVarProducer {
  type OutputPorts = ((String, String),);

  fn produce(&mut self) -> Self::OutputStream {
    let _config = self.config.clone();

    (match &self.filter {
      Some(filter) => {
        let vars: Vec<_> = filter
          .iter()
          .filter_map(|key| std::env::var(key).map(|value| (key.clone(), value)).ok())
          .collect();
        Box::pin(stream::iter(vars)) as Pin<Box<dyn Stream<Item = _> + Send>>
      }
      None => {
        let vars: Vec<_> = std::env::vars().collect();
        Box::pin(stream::iter(vars))
      }
    }) as _
  }

  fn set_config_impl(&mut self, config: ProducerConfig<(String, String)>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<(String, String)> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<(String, String)> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<(String, String)>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<(String, String)>) -> ErrorContext<(String, String)> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "env_var_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "env_var_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

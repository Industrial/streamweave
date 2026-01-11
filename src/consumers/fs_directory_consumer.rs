//! File system directory consumer for creating directories from stream paths.
//!
//! This module provides [`FsDirectoryConsumer`], a consumer that creates directories
//! from path strings in a stream. Each stream item is a path string, and the consumer
//! creates the directory (and any necessary parent directories) using `create_dir_all`.
//!
//! # Overview
//!
//! [`FsDirectoryConsumer`] is useful for creating directory structures dynamically
//! based on stream data. It uses Tokio's async file system operations for efficient
//! I/O and supports configurable error handling.
//!
//! # Key Concepts
//!
//! - **Path Input**: Input must be `String` values representing directory paths
//! - **Recursive Creation**: Uses `create_dir_all` to create parent directories as needed
//! - **Error Handling**: Configurable error strategies for I/O failures
//! - **Async I/O**: Uses Tokio's async file system operations
//!
//! # Core Types
//!
//! - **[`FsDirectoryConsumer`]**: Consumer that creates directories from stream paths
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::FsDirectoryConsumer;
//! use futures::stream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a consumer
//! let mut consumer = FsDirectoryConsumer::new();
//!
//! // Create a stream of directory paths
//! let stream = stream::iter(vec![
//!     "/tmp/dir1".to_string(),
//!     "/tmp/dir2/subdir".to_string(),
//!     "/tmp/dir3".to_string(),
//! ]);
//!
//! // Consume the stream to create directories
//! consumer.consume(Box::pin(stream)).await;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::consumers::FsDirectoryConsumer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a consumer with error handling strategy
//! let consumer = FsDirectoryConsumer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)  // Skip failed creations and continue
//!     .with_name("directory-creator".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Recursive Creation**: Uses `create_dir_all` to automatically create parent
//!   directories, simplifying usage
//! - **Error Logging**: I/O errors are logged but don't stop processing (by design)
//! - **Type Safety**: Requires `String` input to ensure path format
//!
//! # Integration with StreamWeave
//!
//! [`FsDirectoryConsumer`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ConsumerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::fs;
use tracing::warn;

/// A consumer that creates directories from stream items (path strings).
pub struct FsDirectoryConsumer {
  /// Configuration for the consumer
  pub config: ConsumerConfig<String>,
}

impl FsDirectoryConsumer {
  /// Creates a new `FsDirectoryConsumer`.
  pub fn new() -> Self {
    Self {
      config: ConsumerConfig::default(),
    }
  }

  /// Sets the error handling strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this consumer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}

impl Default for FsDirectoryConsumer {
  fn default() -> Self {
    Self::new()
  }
}

impl Input for FsDirectoryConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Consumer for FsDirectoryConsumer {
  type InputPorts = (String,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let component_name = self.config.name.clone();

    while let Some(path) = stream.next().await {
      if let Err(e) = fs::create_dir_all(&path).await {
        warn!(
          component = %component_name,
          path = %path,
          error = %e,
          "Failed to create directory"
        );
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy {
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
      component_name: if self.config.name.is_empty() {
        "fs_directory_consumer".to_string()
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

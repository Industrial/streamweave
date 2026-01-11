//! File system file consumer for writing stream data to files.
//!
//! This module provides [`FsFileConsumer`], a consumer that writes string items
//! from a stream to a file. The file is opened lazily when the first item is consumed,
//! and items are written sequentially with automatic flushing.
//!
//! # Overview
//!
//! [`FsFileConsumer`] is useful for writing stream data to files on disk. It uses
//! Tokio's async file I/O for efficient writes and supports configurable error handling.
//! The file handle is managed internally and opened on first write.
//!
//! # Key Concepts
//!
//! - **Path-Based Writing**: Creates or overwrites a file at the specified path
//! - **Lazy File Opening**: File is opened when the first item is consumed
//! - **Automatic Flushing**: Each write is flushed to ensure data persistence
//! - **Error Handling**: Configurable error strategies for I/O failures
//!
//! # Core Types
//!
//! - **[`FsFileConsumer`]**: Consumer that writes stream items to a file
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::FsFileConsumer;
//! use futures::stream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a consumer with a file path
//! let mut consumer = FsFileConsumer::new("/tmp/output.txt".to_string());
//!
//! // Create a stream of strings to write
//! let stream = stream::iter(vec![
//!     "line 1\n".to_string(),
//!     "line 2\n".to_string(),
//!     "line 3\n".to_string(),
//! ]);
//!
//! // Consume the stream to write to file
//! consumer.consume(Box::pin(stream)).await;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::consumers::FsFileConsumer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a consumer with error handling strategy
//! let consumer = FsFileConsumer::new("/tmp/output.txt".to_string())
//!     .with_error_strategy(ErrorStrategy::Skip)  // Skip failed writes and continue
//!     .with_name("file-writer".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Lazy Opening**: File is opened on first write rather than at creation time,
//!   allowing the consumer to be created before the stream is available
//! - **Automatic Flushing**: Each write is flushed to ensure data is persisted
//!   immediately, important for streaming scenarios
//! - **File Overwrite**: Existing files are overwritten (via `File::create`)
//! - **String Input**: Requires `String` input for file content
//!
//! # Integration with StreamWeave
//!
//! [`FsFileConsumer`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ConsumerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{error, warn};

/// A consumer that writes string items to a file.
///
/// This consumer writes each item from the stream to the specified file path.
/// The file is opened lazily when the first item is consumed.
pub struct FsFileConsumer {
  /// The file handle, opened when the first item is consumed.
  pub file: Option<File>,
  /// The path to the file where items will be written.
  pub path: String,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<String>,
}

impl FsFileConsumer {
  /// Creates a new `FsFileConsumer` with the given file path.
  ///
  /// # Arguments
  ///
  /// * `path` - The path to the file where items will be written.
  pub fn new(path: String) -> Self {
    Self {
      file: None,
      path,
      config: ConsumerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this consumer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
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

// Trait implementations for FsFileConsumer

impl Input for FsFileConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Consumer for FsFileConsumer {
  type InputPorts = (String,);

  /// Consumes a stream and writes each item to the file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be created, an error is logged and all items are dropped.
  /// - If a write fails, an error is logged but consumption continues.
  /// - If a flush fails, a warning is logged but consumption continues.
  ///
  /// Note: The configured error strategy is not currently applied to I/O errors
  /// during stream consumption. This is a known limitation of the current architecture.
  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let component_name = self.config.name.clone();
    let path = self.path.clone();

    // Create file once at the start
    if self.file.is_none() {
      match File::create(&self.path).await {
        Ok(file) => {
          self.file = Some(file);
        }
        Err(e) => {
          error!(
            component = %component_name,
            path = %path,
            error = %e,
            "Failed to create file, all items will be dropped"
          );
        }
      }
    }

    while let Some(value) = stream.next().await {
      if let Some(file) = &mut self.file {
        if let Err(e) = file.write_all(value.as_bytes()).await {
          error!(
            component = %component_name,
            path = %path,
            error = %e,
            "Failed to write to file"
          );
        }
        // Ensure each write is flushed
        if let Err(e) = file.flush().await {
          warn!(
            component = %component_name,
            path = %path,
            error = %e,
            "Failed to flush file"
          );
        }
      }
    }

    // Final flush after stream is consumed
    if let Some(file) = &mut self.file
      && let Err(e) = file.flush().await
    {
      warn!(
        component = %component_name,
        path = %path,
        error = %e,
        "Failed to perform final flush"
      );
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
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
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

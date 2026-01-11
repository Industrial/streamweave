//! File system file producer for reading stream data from files.
//!
//! This module provides [`FsFileProducer`], a producer that reads lines from a file
//! and emits each line as a string item. The file is opened when the stream starts,
//! and lines are read asynchronously using Tokio's async I/O.
//!
//! # Overview
//!
//! [`FsFileProducer`] is useful for reading data from files on disk and converting
//! them into streams. It uses Tokio's async file I/O for efficient reads and supports
//! configurable error handling. The file is opened when production begins.
//!
//! # Key Concepts
//!
//! - **Path-Based Reading**: Reads from a file at the specified path
//! - **Line-Based Reading**: Reads the file line by line, emitting each line as a string
//! - **Async I/O**: Uses Tokio's async file I/O for efficient reading
//! - **Error Handling**: Configurable error strategies for I/O failures
//!
//! # Core Types
//!
//! - **[`FsFileProducer`]**: Producer that reads lines from a file
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::producers::FsFileProducer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that reads from a file
//! let producer = FsFileProducer::new("/path/to/file.txt".to_string());
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
//! use streamweave::producers::FsFileProducer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a producer with error handling strategy
//! let producer = FsFileProducer::new("/path/to/file.txt".to_string())
//!     .with_error_strategy(ErrorStrategy::Skip)  // Skip errors and continue
//!     .with_name("file-reader".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Line-Based**: Reads files line by line, which is the most common use case
//!   for text file processing
//! - **Async I/O**: Uses Tokio's async file I/O for efficient, non-blocking reads
//! - **String Output**: Emits `String` items, one per line from the file
//! - **Error Logging**: I/O errors are logged but don't stop production (empty stream
//!   is returned if file cannot be opened)
//!
//! # Integration with StreamWeave
//!
//! [`FsFileProducer`] implements the [`Producer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ProducerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{error, warn};

/// A producer that reads items from a file.
///
/// This producer reads lines from a file and emits each line as a string item.
pub struct FsFileProducer {
  /// The path to the file to read from.
  pub path: String,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<String>,
}

impl FsFileProducer {
  /// Creates a new `FsFileProducer` with the given file path.
  ///
  /// # Arguments
  ///
  /// * `path` - The path to the file to read from.
  pub fn new(path: String) -> Self {
    Self {
      path,
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

// Trait implementations for FsFileProducer

impl Output for FsFileProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Producer for FsFileProducer {
  type OutputPorts = (String,);

  /// Produces a stream of lines from the file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened, an error is logged and an empty stream is returned.
  /// - If a line cannot be read, a warning is logged and that line is skipped.
  ///
  /// Note: The configured error strategy is not currently applied to I/O errors
  /// during stream production. This is a known limitation of the current architecture.
  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "fs_file_producer".to_string());

    Box::pin(async_stream::stream! {
      match File::open(&path).await {
        Ok(file) => {
          let reader = BufReader::new(file);
          let mut lines = reader.lines();

          loop {
            match lines.next_line().await {
              Ok(Some(line)) => yield line,
              Ok(None) => break,
              Err(e) => {
                warn!(
                  component = %component_name,
                  path = %path,
                  error = %e,
                  "Failed to read line from file, skipping"
                );
                break;
              }
            }
          }
        }
        Err(e) => {
          error!(
            component = %component_name,
            path = %path,
            error = %e,
            "Failed to open file, producing empty stream"
          );
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
        .unwrap_or_else(|| "fs_file_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "fs_file_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

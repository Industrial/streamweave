//! JSON Lines (JSONL) consumer for writing stream data to JSONL files.
//!
//! This module provides [`JsonlConsumer`] and [`JsonlWriteConfig`], a consumer
//! that writes stream items to JSON Lines format. JSON Lines is a text format
//! where each line is a valid JSON value, making it ideal for streaming large
//! datasets and log processing.
//!
//! # Overview
//!
//! [`JsonlConsumer`] is useful for exporting stream data to JSON Lines format,
//! which is commonly used in data processing pipelines, log aggregation, and
//! machine learning workflows. Each stream item is serialized as JSON and written
//! as a single line to the output file.
//!
//! # Key Concepts
//!
//! - **JSON Lines Format**: Each line is a complete, valid JSON value
//! - **Serializable Items**: Items must implement `Serialize` for JSON conversion
//! - **Line-Based Output**: One JSON object per line for easy streaming processing
//! - **Configurable Buffering**: Supports configurable buffer sizes for performance
//!
//! # Core Types
//!
//! - **[`JsonlConsumer<T>`]**: Consumer that writes items to a JSONL file
//! - **[`JsonlWriteConfig`]**: Configuration for JSONL writing behavior
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::JsonlConsumer;
//! use futures::stream;
//! use serde::Serialize;
//!
//! #[derive(Serialize, Clone, Debug)]
//! struct Event {
//!     id: u32,
//!     message: String,
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a consumer with a file path
//! let mut consumer = JsonlConsumer::<Event>::new("events.jsonl");
//!
//! // Create a stream of events
//! let stream = stream::iter(vec![
//!     Event { id: 1, message: "event1".to_string() },
//!     Event { id: 2, message: "event2".to_string() },
//! ]);
//!
//! // Consume the stream (events written to JSONL)
//! consumer.consume(Box::pin(stream)).await;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Configuration
//!
//! ```rust
//! use streamweave::consumers::JsonlConsumer;
//!
//! # use serde::Serialize;
//! # #[derive(Serialize, Clone, Debug)]
//! # struct Event { id: u32, message: String }
//! // Create a consumer with custom configuration
//! let consumer = JsonlConsumer::<Event>::new("events.jsonl")
//!     .with_append(true)  // Append to existing file
//!     .with_buffer_size(16384)  // 16KB buffer
//!     .with_name("event-logger".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Lines Format**: Uses line-delimited JSON for streaming and processing
//!   efficiency
//! - **Serialize Requirement**: Items must implement `Serialize` for flexible
//!   data structure support
//! - **Buffered Writing**: Uses buffered I/O for improved performance
//! - **Lazy File Opening**: File is opened on first write
//!
//! # Integration with StreamWeave
//!
//! [`JsonlConsumer`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ConsumerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde::Serialize;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::{error, warn};

/// Configuration for JSONL writing behavior.
#[derive(Debug, Clone)]
pub struct JsonlWriteConfig {
  /// Whether to append to existing file or overwrite.
  pub append: bool,
  /// Whether to pretty-print JSON (not recommended for JSONL).
  pub pretty: bool,
  /// Buffer size for writing.
  pub buffer_size: usize,
}

impl Default for JsonlWriteConfig {
  fn default() -> Self {
    Self {
      append: false,
      pretty: false,
      buffer_size: 8192,
    }
  }
}

impl JsonlWriteConfig {
  /// Sets whether to append to existing file.
  #[must_use]
  pub fn with_append(mut self, append: bool) -> Self {
    self.append = append;
    self
  }

  /// Sets the buffer size.
  #[must_use]
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.buffer_size = size;
    self
  }
}

/// A consumer that writes JSON Lines (JSONL) files.
///
/// JSON Lines is a text format where each line is a valid JSON value.
/// This consumer serializes each input element as JSON and writes it
/// as a single line to the output file.
///
/// # Example
///
/// ```ignore
/// use streamweave_jsonl_consumer::JsonlConsumer;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let consumer = JsonlConsumer::<Event>::new("events.jsonl");
/// ```
pub struct JsonlConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Path to the JSONL file.
  pub path: PathBuf,
  /// Consumer configuration.
  pub config: ConsumerConfig<T>,
  /// JSONL-specific configuration.
  pub jsonl_config: JsonlWriteConfig,
  /// File handle (opened on first write).
  pub file: Option<BufWriter<tokio::fs::File>>,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> JsonlConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new JSONL consumer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      config: ConsumerConfig::default(),
      jsonl_config: JsonlWriteConfig::default(),
      file: None,
      _phantom: PhantomData,
    }
  }

  /// Sets the error strategy for the consumer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the consumer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  /// Sets whether to append to existing file.
  #[must_use]
  pub fn with_append(mut self, append: bool) -> Self {
    self.jsonl_config.append = append;
    self
  }

  /// Sets the buffer size.
  #[must_use]
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.jsonl_config.buffer_size = size;
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &PathBuf {
    &self.path
  }
}

// Trait implementations for JsonlConsumer

impl<T> Input for JsonlConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

#[async_trait]
impl<T> Consumer for JsonlConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  /// Consumes a stream and writes each item as a JSON line to the file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened, an error is logged and no data is written.
  /// - If an item cannot be serialized or written, the error strategy determines the action.
  async fn consume(&mut self, input: Self::InputStream) {
    let path = self.path.clone();
    let component_name = if self.config.name.is_empty() {
      "jsonl_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let append = self.jsonl_config.append;
    let buffer_size = self.jsonl_config.buffer_size;

    // Open the file
    let file_result = OpenOptions::new()
      .create(true)
      .write(true)
      .append(append)
      .truncate(!append)
      .open(&path)
      .await;

    let file = match file_result {
      Ok(f) => f,
      Err(e) => {
        error!(
          component = %component_name,
          path = %path.display(),
          error = %e,
          "Failed to open file for writing"
        );
        return;
      }
    };

    let mut writer = BufWriter::with_capacity(buffer_size, file);
    let mut input = std::pin::pin!(input);

    while let Some(item) = input.next().await {
      // Serialize the item to JSON
      let json_result = serde_json::to_string(&item);

      match json_result {
        Ok(json) => {
          // Write the JSON line
          if let Err(e) = writer.write_all(json.as_bytes()).await {
            let error = StreamError::new(
              Box::new(e),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(item.clone()),
                component_name: component_name.clone(),
                component_type: std::any::type_name::<Self>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<Self>().to_string(),
              },
            );
            match self.handle_error(&error) {
              ErrorAction::Stop => {
                error!(
                  component = %component_name,
                  error = %error,
                  "Stopping due to write error"
                );
                break;
              }
              ErrorAction::Skip => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Skipping item due to write error"
                );
                continue;
              }
              ErrorAction::Retry => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Retry not fully supported for write errors, skipping"
                );
                continue;
              }
            }
          }

          // Write newline
          if let Err(e) = writer.write_all(b"\n").await {
            let error = StreamError::new(
              Box::new(e),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(item),
                component_name: component_name.clone(),
                component_type: std::any::type_name::<Self>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<Self>().to_string(),
              },
            );
            match self.handle_error(&error) {
              ErrorAction::Stop => {
                error!(
                  component = %component_name,
                  error = %error,
                  "Stopping due to newline write error"
                );
                break;
              }
              ErrorAction::Skip | ErrorAction::Retry => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Error writing newline, continuing"
                );
              }
            }
          }
        }
        Err(e) => {
          let error = StreamError::new(
            Box::new(e),
            ErrorContext {
              timestamp: chrono::Utc::now(),
              item: Some(item),
              component_name: component_name.clone(),
              component_type: std::any::type_name::<Self>().to_string(),
            },
            ComponentInfo {
              name: component_name.clone(),
              type_name: std::any::type_name::<Self>().to_string(),
            },
          );
          match self.handle_error(&error) {
            ErrorAction::Stop => {
              error!(
                component = %component_name,
                error = %error,
                "Stopping due to serialization error"
              );
              break;
            }
            ErrorAction::Skip => {
              warn!(
                component = %component_name,
                error = %error,
                "Skipping item due to serialization error"
              );
            }
            ErrorAction::Retry => {
              warn!(
                component = %component_name,
                error = %error,
                "Retry not supported for serialization errors, skipping"
              );
            }
          }
        }
      }
    }

    // Flush the writer
    if let Err(e) = writer.flush().await {
      error!(
        component = %component_name,
        error = %e,
        "Failed to flush file"
      );
    }

    // Store the file handle
    self.file = Some(writer);
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

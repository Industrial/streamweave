//! CSV write transformer for writing data to CSV files while passing through.
//!
//! This module provides [`CsvWriteTransformer<T>`], a transformer that writes
//! serializable data to CSV files while passing the same data through to the output
//! stream. This enables persisting intermediate results to CSV format while continuing
//! the main pipeline flow.
//!
//! # Overview
//!
//! [`CsvWriteTransformer`] writes items to a CSV file while also passing them through
//! to the output stream. This is useful for logging intermediate results, creating
//! backup copies, or debugging pipeline execution without interrupting the data flow.
//!
//! # Key Concepts
//!
//! - **CSV Writing**: Writes serializable data to CSV files
//! - **Pass-Through**: Outputs the same data that was written (enables chaining)
//! - **Header Support**: Optional CSV header row writing
//! - **Configuration**: Configurable delimiter, quote character, and other CSV options
//! - **File I/O**: Uses synchronous file I/O with mutex for thread-safe writes
//!
//! # Core Types
//!
//! - **[`CsvWriteTransformer<T>`]**: Transformer that writes data to CSV files
//! - **[`CsvWriteTransformerConfig`]**: Configuration for CSV writing behavior
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use streamweave::transformers::CsvWriteTransformer;
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct Record {
//!     name: String,
//!     age: u32,
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that writes to output.csv
//! let transformer = CsvWriteTransformer::<Record>::new("output.csv");
//!
//! // Input: [Record { name: "Alice", age: 30 }, ...]
//! // Writes to CSV and outputs: [Record { name: "Alice", age: 30 }, ...]
//! # Ok(())
//! # }
//! ```
//!
//! ## With Configuration
//!
//! ```rust,no_run
//! use streamweave::transformers::CsvWriteTransformer;
//!
//! // Create with custom delimiter and no headers
//! let transformer = CsvWriteTransformer::<MyType>::new("output.csv")
//!     .with_headers(false)
//!     .with_delimiter(b';');
//! ```
//!
//! # Design Decisions
//!
//! - **Pass-Through Pattern**: Writes data and passes it through for pipeline chaining
//! - **Synchronous I/O**: Uses synchronous file I/O with mutex for thread safety
//! - **CSV Library**: Uses the `csv` crate for robust CSV writing
//! - **Serialization**: Requires `Serialize` trait for CSV conversion
//! - **Shared Writer**: Uses `Arc<Mutex<>>` for thread-safe shared writer state
//!
//! # Integration with StreamWeave
//!
//! [`CsvWriteTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use csv::{Writer, WriterBuilder};
use futures::{Stream, StreamExt};
use serde::Serialize;
use std::fs::File;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tracing::{error, warn};

/// Configuration for CSV writing behavior in transformers.
#[derive(Debug, Clone)]
pub struct CsvWriteTransformerConfig {
  /// Whether to write a header row.
  pub write_headers: bool,
  /// The delimiter character (default: comma).
  pub delimiter: u8,
  /// The quote character (default: double quote).
  pub quote: u8,
  /// Whether double quotes are used for escaping.
  pub double_quote: bool,
  /// Whether to flush after each record.
  pub flush_on_write: bool,
}

impl Default for CsvWriteTransformerConfig {
  fn default() -> Self {
    Self {
      write_headers: true,
      delimiter: b',',
      quote: b'"',
      double_quote: true,
      flush_on_write: false,
    }
  }
}

impl CsvWriteTransformerConfig {
  /// Sets whether to write a header row.
  #[must_use]
  pub fn with_headers(mut self, write_headers: bool) -> Self {
    self.write_headers = write_headers;
    self
  }

  /// Sets the delimiter character.
  #[must_use]
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.delimiter = delimiter;
    self
  }

  /// Sets whether to flush after each record.
  #[must_use]
  pub fn with_flush_on_write(mut self, flush: bool) -> Self {
    self.flush_on_write = flush;
    self
  }
}

/// A transformer that writes data to CSV files while passing data through.
///
/// Each input item is written to the CSV file, and then the same item is output,
/// enabling writing intermediate results while continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::CsvWriteTransformer;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Record {
///     name: String,
///     age: u32,
/// }
///
/// let transformer = CsvWriteTransformer::<Record>::new("output.csv");
/// // Input: [Record { name: "Alice", age: 30 }, ...]
/// // Writes to CSV and outputs: [Record { name: "Alice", age: 30 }, ...]
/// ```
pub struct CsvWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Path to the CSV file.
  path: PathBuf,
  /// CSV-specific configuration.
  csv_config: CsvWriteTransformerConfig,
  /// Transformer configuration.
  config: TransformerConfig<T>,
  /// Writer handle (shared across stream items).
  writer: Arc<Mutex<Option<Writer<File>>>>,
  /// Whether we've written the first record (for header handling).
  first_record_written: Arc<Mutex<bool>>,
}

impl<T> CsvWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `CsvWriteTransformer` for the specified file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the CSV file to write.
  #[must_use]
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      csv_config: CsvWriteTransformerConfig::default(),
      config: TransformerConfig::default(),
      writer: Arc::new(Mutex::new(None)),
      first_record_written: Arc::new(Mutex::new(false)),
    }
  }

  /// Sets the error handling strategy for this transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Sets whether to write a header row.
  #[must_use]
  pub fn with_headers(mut self, write_headers: bool) -> Self {
    self.csv_config.write_headers = write_headers;
    self
  }

  /// Sets the delimiter character.
  #[must_use]
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.csv_config.delimiter = delimiter;
    self
  }

  /// Sets whether to flush after each record.
  #[must_use]
  pub fn with_flush_on_write(mut self, flush: bool) -> Self {
    self.csv_config.flush_on_write = flush;
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &PathBuf {
    &self.path
  }
}

impl<T> Clone for CsvWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      csv_config: self.csv_config.clone(),
      config: self.config.clone(),
      writer: Arc::clone(&self.writer),
      first_record_written: Arc::clone(&self.first_record_written),
    }
  }
}

impl<T> Input for CsvWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for CsvWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for CsvWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "csv_write_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let csv_config = self.csv_config.clone();
    let writer = Arc::clone(&self.writer);
    let first_record_written = Arc::clone(&self.first_record_written);

    Box::pin(async_stream::stream! {
      // Open the file on first write
      {
        let mut writer_guard = writer.lock().unwrap();
        if writer_guard.is_none() {
          let file = match File::create(&path) {
            Ok(f) => f,
            Err(e) => {
              error!(
                component = %component_name,
                path = %path.display(),
                error = %e,
                "Failed to create CSV file for writing"
              );
              return;
            }
          };

          let csv_writer = WriterBuilder::new()
            .has_headers(csv_config.write_headers)
            .delimiter(csv_config.delimiter)
            .quote(csv_config.quote)
            .double_quote(csv_config.double_quote)
            .from_writer(file);

          *writer_guard = Some(csv_writer);
        }
      }

      let mut input_stream = input;
      while let Some(item) = input_stream.next().await {
        // Write to CSV
        {
          let mut writer_guard = writer.lock().unwrap();
          if let Some(ref mut w) = *writer_guard {
            if let Err(e) = w.serialize(&item) {
              let stream_error = StreamError::new(
                Box::new(e),
                ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: Some(item.clone()),
                  component_name: component_name.clone(),
                  component_type: std::any::type_name::<CsvWriteTransformer<T>>().to_string(),
                },
                ComponentInfo {
                  name: component_name.clone(),
                  type_name: std::any::type_name::<CsvWriteTransformer<T>>().to_string(),
                },
              );

              match handle_error_strategy(&error_strategy, &stream_error) {
                ErrorAction::Stop => {
                  error!(
                    component = %component_name,
                    error = %stream_error,
                    "Stopping due to CSV serialization error"
                  );
                  return;
                }
                ErrorAction::Skip => {
                  warn!(
                    component = %component_name,
                    error = %stream_error,
                    "Skipping item due to CSV serialization error"
                  );
                  continue;
                }
                ErrorAction::Retry => {
                  warn!(
                    component = %component_name,
                    error = %stream_error,
                    "Retry not supported for CSV serialization errors, skipping"
                  );
                  continue;
                }
              }
            }

            // Flush if configured
            if csv_config.flush_on_write
              && let Err(e) = w.flush() {
                warn!(
                  component = %component_name,
                  error = %e,
                  "Failed to flush CSV writer"
                );
              }

            *first_record_written.lock().unwrap() = true;
          }
        }

        // Pass through the item
        yield item;
      }

      // Final flush
      {
        let mut writer_guard = writer.lock().unwrap();
        if let Some(ref mut w) = *writer_guard
          && let Err(e) = w.flush() {
            error!(
              component = %component_name,
              error = %e,
              "Failed to flush CSV file"
            );
          }
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
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
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "csv_write_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "csv_write_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

/// Helper function to handle error strategy
pub(crate) fn handle_error_strategy<T>(
  strategy: &ErrorStrategy<T>,
  error: &StreamError<T>,
) -> ErrorAction
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
  match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    ErrorStrategy::Skip => ErrorAction::Skip,
    ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}

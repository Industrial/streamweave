//! CSV read transformer for reading and deserializing CSV files from file paths.
//!
//! This module provides [`CsvReadTransformer<T>`], a transformer that reads CSV files
//! from input file paths and deserializes them into strongly-typed Rust values.
//! It supports processing multiple CSV files in a single pipeline.
//!
//! # Overview
//!
//! [`CsvReadTransformer`] takes file paths (String) as input and outputs deserialized
//! CSV rows (T). It enables processing multiple CSV files in a pipeline, reading each
//! file and streaming its rows. This is useful for batch processing, file aggregation,
//! and multi-file data pipelines.
//!
//! # Key Concepts
//!
//! - **CSV File Reading**: Reads and parses CSV files from file system
//! - **Type-Safe Deserialization**: Deserializes CSV rows into strongly-typed Rust types
//! - **Multiple Files**: Processes multiple CSV files from a stream of file paths
//! - **Configuration**: Configurable delimiter, headers, trimming, and other CSV options
//! - **Error Handling**: Configurable error strategies for parse failures
//!
//! # Core Types
//!
//! - **[`CsvReadTransformer<T>`]**: Transformer that reads CSV files from file paths
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use streamweave::transformers::CsvReadTransformer;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize, Clone, Debug)]
//! struct Record {
//!     name: String,
//!     age: u32,
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let transformer = CsvReadTransformer::<Record>::new();
//! // Input: ["file1.csv", "file2.csv"]
//! // Output: [Record { name: "Alice", age: 30 }, Record { name: "Bob", age: 25 }, ...]
//! # Ok(())
//! # }
//! ```
//!
//! # Design Decisions
//!
//! - **Type-Safe**: Uses `DeserializeOwned` for type-safe CSV deserialization
//! - **CSV Library**: Uses the `csv` crate for robust CSV parsing
//! - **Blocking I/O**: Uses `spawn_blocking` for synchronous file I/O in async context
//! - **Multiple Files**: Supports processing multiple CSV files from a stream
//! - **Configuration**: Reuses `CsvReadConfig` from producers for consistency
//!
//! # Integration with StreamWeave
//!
//! [`CsvReadTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::producers::CsvReadConfig;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use csv::Trim;
use futures::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::BufReader;
use std::marker::PhantomData;
use std::pin::Pin;
use tracing::error;

/// A transformer that reads CSV files from file paths and outputs parsed rows.
///
/// This transformer takes file paths (String) as input and outputs deserialized
/// CSV rows (T). It enables processing multiple CSV files in a pipeline.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::CsvReadTransformer;
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Clone, Debug)]
/// struct Record {
///     name: String,
///     age: u32,
/// }
///
/// let transformer = CsvReadTransformer::<Record>::new();
/// // Input: ["file1.csv", "file2.csv"]
/// // Output: [Record { name: "Alice", age: 30 }, Record { name: "Bob", age: 25 }, ...]
/// ```
pub struct CsvReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// CSV-specific configuration.
  pub csv_config: CsvReadConfig,
  /// Transformer configuration.
  pub config: TransformerConfig<String>,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> CsvReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Creates a new `CsvReadTransformer` with default configuration.
  pub fn new() -> Self {
    Self {
      csv_config: CsvReadConfig::default(),
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }

  /// Sets whether the CSV has a header row.
  ///
  /// # Arguments
  ///
  /// * `has_headers` - Whether the CSV has a header row.
  pub fn with_headers(mut self, has_headers: bool) -> Self {
    self.csv_config.has_headers = has_headers;
    self
  }

  /// Sets the delimiter character.
  ///
  /// # Arguments
  ///
  /// * `delimiter` - The delimiter character.
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.csv_config.delimiter = delimiter;
    self
  }

  /// Sets whether to allow flexible column counts.
  ///
  /// # Arguments
  ///
  /// * `flexible` - Whether to allow flexible column counts.
  pub fn with_flexible(mut self, flexible: bool) -> Self {
    self.csv_config.flexible = flexible;
    self
  }

  /// Sets whether to trim whitespace from fields.
  ///
  /// # Arguments
  ///
  /// * `trim` - Whether to trim whitespace from fields.
  pub fn with_trim(mut self, trim: bool) -> Self {
    self.csv_config.trim = trim;
    self
  }

  /// Sets the comment character.
  ///
  /// # Arguments
  ///
  /// * `comment` - The comment character (None means no comments).
  pub fn with_comment(mut self, comment: Option<u8>) -> Self {
    self.csv_config.comment = comment;
    self
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Default for CsvReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for CsvReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn clone(&self) -> Self {
    Self {
      csv_config: self.csv_config.clone(),
      config: self.config.clone(),
      _phantom: PhantomData,
    }
  }
}

impl<T> Input for CsvReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl<T> Output for CsvReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for CsvReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type InputPorts = (String,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let csv_config = self.csv_config.clone();
    let component_name_base = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "csv_read_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();

    Box::pin(async_stream::stream! {
      let mut input = input;
      let component_name = component_name_base.clone();

      while let Some(path) = input.next().await {
        // Read CSV file for this path
        let csv_config = csv_config.clone();
        let component_name_for_closure = component_name.clone();
        let error_strategy = error_strategy.clone();

        // Use tokio's blocking spawn for synchronous CSV reading
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<T, StreamError<String>>>(100);

        let path_clone = path.clone();

        let handle = tokio::task::spawn_blocking(move || {
          let file = match File::open(&path_clone) {
            Ok(f) => f,
            Err(e) => {
              error!(
                component = %component_name_for_closure,
                path = %path_clone,
                error = %e,
                "Failed to open CSV file"
              );
              return;
            }
          };

          let reader = BufReader::new(file);

          let mut csv_reader = csv::ReaderBuilder::new()
            .has_headers(csv_config.has_headers)
            .delimiter(csv_config.delimiter)
            .flexible(csv_config.flexible)
            .trim(if csv_config.trim { Trim::All } else { Trim::None })
            .quote(csv_config.quote)
            .double_quote(csv_config.double_quote)
            .from_reader(reader);

          if let Some(comment) = csv_config.comment {
            csv_reader = csv::ReaderBuilder::new()
              .has_headers(csv_config.has_headers)
              .delimiter(csv_config.delimiter)
              .flexible(csv_config.flexible)
              .trim(if csv_config.trim { Trim::All } else { Trim::None })
              .quote(csv_config.quote)
              .double_quote(csv_config.double_quote)
              .comment(Some(comment))
              .from_reader(BufReader::new(File::open(&path_clone).unwrap()));
          }

          for result in csv_reader.deserialize::<T>() {
            match result {
              Ok(record) => {
                if tx.blocking_send(Ok(record)).is_err() {
                  break;
                }
              }
              Err(e) => {
                let stream_error = StreamError::new(
                  Box::new(e),
                  ErrorContext {
                    timestamp: chrono::Utc::now(),
                    item: Some(path_clone.clone()),
                    component_name: component_name_for_closure.clone(),
                    component_type: std::any::type_name::<CsvReadTransformer<T>>().to_string(),
                  },
                  ComponentInfo {
                    name: component_name_for_closure.clone(),
                    type_name: std::any::type_name::<CsvReadTransformer<T>>().to_string(),
                  },
                );

                let action = match &error_strategy {
                  ErrorStrategy::Stop => ErrorAction::Stop,
                  ErrorStrategy::Skip => ErrorAction::Skip,
                  ErrorStrategy::Retry(n) if stream_error.retries < *n => ErrorAction::Retry,
                  ErrorStrategy::Custom(handler) => handler(&stream_error),
                  _ => ErrorAction::Stop,
                };

                match action {
                  ErrorAction::Stop => {
                    let _ = tx.blocking_send(Err(stream_error));
                    break;
                  }
                  ErrorAction::Skip => {
                    // Continue to next record
                  }
                  ErrorAction::Retry => {
                    // Retry not supported for CSV row reading, skip
                  }
                }
              }
            }
          }
        });

        // Receive and yield records from this file
        while let Some(result) = rx.recv().await {
          match result {
            Ok(record) => yield record,
            Err(stream_error) => {
              error!(
                component = %component_name,
                error = %stream_error,
                "Error reading CSV record"
              );
              break;
            }
          }
        }

        // Wait for the blocking task to finish
        let _ = handle.await;
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
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
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "csv_read_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

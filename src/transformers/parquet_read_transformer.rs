//! Parquet read transformer for reading Parquet files in StreamWeave pipelines.
//!
//! This module provides [`ParquetReadTransformer`], a transformer that reads
//! Parquet files from input file paths and produces Arrow RecordBatches as output.
//! Parquet is a columnar storage format optimized for analytics workloads, and
//! this transformer enables reading Parquet data in streaming pipelines.
//!
//! # Overview
//!
//! [`ParquetReadTransformer`] is useful for reading analytics data from Parquet
//! files in StreamWeave pipelines. It reads Parquet files from file system paths
//! and produces Arrow RecordBatches for efficient processing, supporting batch
//! processing for memory efficiency.
//!
//! # Key Concepts
//!
//! - **Parquet File Reading**: Reads Parquet files from file system paths
//! - **Arrow Integration**: Produces Arrow RecordBatches for efficient processing
//! - **Batch Processing**: Reads data in batches for memory efficiency
//! - **Columnar Storage**: Leverages Parquet's columnar format for analytics workloads
//! - **Error Handling**: Configurable error strategies for file read failures
//!
//! # Core Types
//!
//! - **[`ParquetReadTransformer`]**: Transformer that reads Parquet files and outputs Arrow RecordBatches
//! - **[`ParquetReadConfig`]**: Configuration for Parquet reading behavior
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::ParquetReadTransformer;
//!
//! // Create a Parquet read transformer
//! let transformer = ParquetReadTransformer::new();
//! ```
//!
//! ## With Custom Batch Size
//!
//! ```rust
//! use streamweave::transformers::ParquetReadTransformer;
//!
//! // Create a transformer with custom batch size
//! let transformer = ParquetReadTransformer::new()
//!     .with_batch_size(1000);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::ParquetReadTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = ParquetReadTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("parquet-reader".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Arrow Integration**: Uses Arrow RecordBatches for efficient columnar data
//!   processing
//! - **Batch Processing**: Reads data in configurable batches for memory efficiency
//! - **Parquet Format**: Leverages Parquet's columnar storage format optimized for
//!   analytics workloads
//! - **File Path Input**: Takes file paths as input strings for flexible file access
//!
//! # Integration with StreamWeave
//!
//! [`ParquetReadTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::producers::ParquetReadConfig;
use crate::{Input, Output, Transformer, TransformerConfig};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::fs::File;
use std::pin::Pin;
use tracing::error;

/// A transformer that reads Parquet files from input paths and outputs Arrow RecordBatches.
///
/// Input: String (file path)
/// Output: RecordBatch (Arrow RecordBatch)
///
/// Parquet is a columnar storage format that's efficient for analytics workloads.
/// This transformer reads data in batches and yields Arrow RecordBatch objects.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::ParquetReadTransformer;
///
/// let transformer = ParquetReadTransformer::new();
/// // Input: ["data1.parquet", "data2.parquet"]
/// // Output: [RecordBatch, RecordBatch, ...]
/// ```
pub struct ParquetReadTransformer {
  /// Transformer configuration.
  pub config: TransformerConfig<String>,
  /// Parquet-specific configuration.
  pub parquet_config: ParquetReadConfig,
}

impl ParquetReadTransformer {
  /// Creates a new `ParquetReadTransformer`.
  #[must_use]
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
      parquet_config: ParquetReadConfig::default(),
    }
  }

  /// Sets the error strategy for the transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Sets the batch size for reading.
  ///
  /// # Arguments
  ///
  /// * `batch_size` - Number of rows per batch.
  pub fn with_batch_size(mut self, batch_size: usize) -> Self {
    self.parquet_config = self.parquet_config.with_batch_size(batch_size);
    self
  }

  /// Sets the column projection.
  ///
  /// # Arguments
  ///
  /// * `projection` - Column indices to read (None means read all columns).
  pub fn with_projection(mut self, projection: Vec<usize>) -> Self {
    self.parquet_config = self.parquet_config.with_projection(projection);
    self
  }

  /// Sets the row groups to read.
  ///
  /// # Arguments
  ///
  /// * `row_groups` - Row group indices to read (None means read all).
  pub fn with_row_groups(mut self, row_groups: Vec<usize>) -> Self {
    self.parquet_config = self.parquet_config.with_row_groups(row_groups);
    self
  }
}

impl Default for ParquetReadTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ParquetReadTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      parquet_config: self.parquet_config.clone(),
    }
  }
}

impl Input for ParquetReadTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl Output for ParquetReadTransformer {
  type Output = RecordBatch;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Transformer for ParquetReadTransformer {
  type InputPorts = (String,);
  type OutputPorts = (RecordBatch,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "parquet_read_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let parquet_config = self.parquet_config.clone();

    Box::pin(input.flat_map(move |path| {
      let component_name_clone = component_name.clone();
      let error_strategy_clone = error_strategy.clone();
      let parquet_config_clone = parquet_config.clone();

      // Use async_stream to handle the async file reading and parsing
      async_stream::stream! {
        // Parquet reading is synchronous, so we use blocking_task wrapper
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<RecordBatch, StreamError<String>>>(100);

        let component_name_blocking = component_name_clone.clone();
        let error_strategy_blocking = error_strategy_clone.clone();
        let parquet_config_blocking = parquet_config_clone.clone();
        let path_blocking = path.clone();

        let handle = tokio::task::spawn_blocking(move || {
          let file = match File::open(&path_blocking) {
            Ok(f) => f,
            Err(e) => {
              error!(
                component = %component_name_blocking,
                path = %path_blocking,
                error = %e,
                "Failed to open Parquet file"
              );
              let stream_error = StreamError::new(
                Box::new(e),
                ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: Some(path_blocking.clone()),
                  component_name: component_name_blocking.clone(),
                  component_type: std::any::type_name::<ParquetReadTransformer>().to_string(),
                },
                ComponentInfo {
                  name: component_name_blocking.clone(),
                  type_name: std::any::type_name::<ParquetReadTransformer>().to_string(),
                },
              );
              let _ = tx.blocking_send(Err(stream_error));
              return;
            }
          };

          let builder = match ParquetRecordBatchReaderBuilder::try_new(file) {
            Ok(b) => b,
            Err(e) => {
              error!(
                component = %component_name_blocking,
                path = %path_blocking,
                error = %e,
                "Failed to create Parquet reader"
              );
              let stream_error = StreamError::new(
                Box::new(e),
                ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: Some(path_blocking.clone()),
                  component_name: component_name_blocking.clone(),
                  component_type: std::any::type_name::<ParquetReadTransformer>().to_string(),
                },
                ComponentInfo {
                  name: component_name_blocking.clone(),
                  type_name: std::any::type_name::<ParquetReadTransformer>().to_string(),
                },
              );
              let _ = tx.blocking_send(Err(stream_error));
              return;
            }
          };

          // Apply configuration
          let mut builder = builder.with_batch_size(parquet_config_blocking.batch_size);

          if let Some(ref projection) = parquet_config_blocking.projection {
            // Create projection mask from indices
            let projection_mask = parquet::arrow::ProjectionMask::roots(
              builder.parquet_schema(),
              projection.iter().copied()
            );
            builder = builder.with_projection(projection_mask);
          }

          if let Some(ref row_groups) = parquet_config_blocking.row_groups {
            builder = builder.with_row_groups(row_groups.clone());
          }

          let reader = match builder.build() {
            Ok(r) => r,
            Err(e) => {
              error!(
                component = %component_name_blocking,
                path = %path_blocking,
                error = %e,
                "Failed to build Parquet reader"
              );
              let stream_error = StreamError::new(
                Box::new(e),
                ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: Some(path_blocking.clone()),
                  component_name: component_name_blocking.clone(),
                  component_type: std::any::type_name::<ParquetReadTransformer>().to_string(),
                },
                ComponentInfo {
                  name: component_name_blocking.clone(),
                  type_name: std::any::type_name::<ParquetReadTransformer>().to_string(),
                },
              );
              let _ = tx.blocking_send(Err(stream_error));
              return;
            }
          };

          for result in reader {
            match result {
              Ok(batch) => {
                if tx.blocking_send(Ok(batch)).is_err() {
                  break;
                }
              }
              Err(e) => {
                let stream_error = StreamError::new(
                  Box::new(e),
                  ErrorContext {
                    timestamp: chrono::Utc::now(),
                    item: Some(path_blocking.clone()),
                    component_name: component_name_blocking.clone(),
                    component_type: std::any::type_name::<ParquetReadTransformer>().to_string(),
                  },
                  ComponentInfo {
                    name: component_name_blocking.clone(),
                    type_name: std::any::type_name::<ParquetReadTransformer>().to_string(),
                  },
                );

                match handle_error_strategy(&error_strategy_blocking, &stream_error) {
                  ErrorAction::Stop => {
                    let _ = tx.blocking_send(Err(stream_error));
                    break;
                  }
                  ErrorAction::Skip => {
                    // Continue to next batch
                  }
                  ErrorAction::Retry => {
                    // Retry not supported for Parquet batch reading, skip
                  }
                }
              }
            }
          }
        });

        // Convert the mpsc receiver into a stream
        while let Some(result) = rx.recv().await {
          match result {
            Ok(batch) => yield batch,
            Err(stream_error) => {
              error!(
                component = %component_name_clone,
                path = %path,
                error = %stream_error,
                "Error reading Parquet batch from channel"
              );
              match handle_error_strategy(&error_strategy_clone, &stream_error) {
                ErrorAction::Stop => {
                  // Stop processing this file
                  break;
                }
                ErrorAction::Skip => {
                  // Skip this file and continue
                }
                ErrorAction::Retry => {
                  // Retry not directly supported for file read errors
                }
              }
            }
          }
        }

        // Wait for the blocking task to finish
        let _ = handle.await;
      }
    }))
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
        .unwrap_or_else(|| "parquet_read_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "parquet_read_transformer".to_string()),
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

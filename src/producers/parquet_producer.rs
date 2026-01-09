use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use futures::Stream;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::fs::File;
use std::pin::Pin;
use tracing::error;

/// Configuration for Parquet reading behavior.
#[derive(Debug, Clone)]
pub struct ParquetReadConfig {
  /// Batch size for reading (number of rows per batch).
  pub batch_size: usize,
  /// Column projection (None means read all columns).
  pub projection: Option<Vec<usize>>,
  /// Row groups to read (None means read all).
  pub row_groups: Option<Vec<usize>>,
}

impl Default for ParquetReadConfig {
  fn default() -> Self {
    Self {
      batch_size: 1024,
      projection: None,
      row_groups: None,
    }
  }
}

impl ParquetReadConfig {
  /// Sets the batch size.
  #[must_use]
  pub fn with_batch_size(mut self, batch_size: usize) -> Self {
    self.batch_size = batch_size;
    self
  }

  /// Sets the column projection.
  #[must_use]
  pub fn with_projection(mut self, projection: Vec<usize>) -> Self {
    self.projection = Some(projection);
    self
  }

  /// Sets the row groups to read.
  #[must_use]
  pub fn with_row_groups(mut self, row_groups: Vec<usize>) -> Self {
    self.row_groups = Some(row_groups);
    self
  }
}

/// A producer that reads Apache Parquet files and produces Arrow RecordBatches.
///
/// Parquet is a columnar storage format that's efficient for analytics workloads.
/// This producer reads data in batches and yields Arrow RecordBatch objects.
///
/// # Example
///
/// ```ignore
/// use crate::producers::ParquetProducer;
///
/// let producer = ParquetProducer::new("data.parquet");
/// ```
pub struct ParquetProducer {
  /// Path to the Parquet file.
  pub path: String,
  /// Producer configuration.
  pub config: ProducerConfig<RecordBatch>,
  /// Parquet-specific configuration.
  pub parquet_config: ParquetReadConfig,
}

impl ParquetProducer {
  /// Creates a new Parquet producer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<String>) -> Self {
    Self {
      path: path.into(),
      config: ProducerConfig::default(),
      parquet_config: ParquetReadConfig::default(),
    }
  }

  /// Sets the error strategy for the producer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<RecordBatch>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the producer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Sets the batch size.
  #[must_use]
  pub fn with_batch_size(mut self, batch_size: usize) -> Self {
    self.parquet_config.batch_size = batch_size;
    self
  }

  /// Sets the column projection.
  #[must_use]
  pub fn with_projection(mut self, projection: Vec<usize>) -> Self {
    self.parquet_config.projection = Some(projection);
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &str {
    &self.path
  }
}

impl Clone for ParquetProducer {
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      config: self.config.clone(),
      parquet_config: self.parquet_config.clone(),
    }
  }
}

// Trait implementations for ParquetProducer

impl Output for ParquetProducer {
  type Output = RecordBatch;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Producer for ParquetProducer {
  type OutputPorts = (RecordBatch,);

  /// Produces a stream of Arrow RecordBatches from a Parquet file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened or is not valid Parquet, an error is logged
  ///   and an empty stream is returned.
  /// - If a batch cannot be read, the error strategy determines the action.
  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "parquet_producer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let parquet_config = self.parquet_config.clone();

    // Parquet reading is synchronous, so we use blocking_task wrapper
    Box::pin(async_stream::stream! {
      let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<RecordBatch, StreamError<RecordBatch>>>(100);

      let component_name_clone = component_name.clone();
      let error_strategy_clone = error_strategy.clone();

      let handle = tokio::task::spawn_blocking(move || {
        let file = match File::open(&path) {
          Ok(f) => f,
          Err(e) => {
            error!(
              component = %component_name_clone,
              path = %path,
              error = %e,
              "Failed to open Parquet file"
            );
            return;
          }
        };

        let builder = match ParquetRecordBatchReaderBuilder::try_new(file) {
          Ok(b) => b,
          Err(e) => {
            error!(
              component = %component_name_clone,
              path = %path,
              error = %e,
              "Failed to create Parquet reader"
            );
            return;
          }
        };

        // Apply configuration
        let mut builder = builder.with_batch_size(parquet_config.batch_size);

        if let Some(ref projection) = parquet_config.projection {
          // Create projection mask from indices
          let projection_mask = parquet::arrow::ProjectionMask::roots(
            builder.parquet_schema(),
            projection.iter().copied()
          );
          builder = builder.with_projection(projection_mask);
        }

        if let Some(ref row_groups) = parquet_config.row_groups {
          builder = builder.with_row_groups(row_groups.clone());
        }

        let reader = match builder.build() {
          Ok(r) => r,
          Err(e) => {
            error!(
              component = %component_name_clone,
              path = %path,
              error = %e,
              "Failed to build Parquet reader"
            );
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
                  item: None,
                  component_name: component_name_clone.clone(),
                  component_type: std::any::type_name::<ParquetProducer>().to_string(),
                },
                ComponentInfo {
                  name: component_name_clone.clone(),
                  type_name: std::any::type_name::<ParquetProducer>().to_string(),
                },
              );

              match handle_error_strategy(&error_strategy_clone, &stream_error) {
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

      while let Some(result) = rx.recv().await {
        match result {
          Ok(batch) => yield batch,
          Err(stream_error) => {
            error!(
              component = %component_name,
              error = %stream_error,
              "Error reading Parquet batch"
            );
            break;
          }
        }
      }

      // Wait for the blocking task to finish
      let _ = handle.await;
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<RecordBatch>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<RecordBatch> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<RecordBatch> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<RecordBatch>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<RecordBatch>) -> ErrorContext<RecordBatch> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "parquet_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "parquet_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

/// Helper function to handle error strategy
fn handle_error_strategy(
  strategy: &ErrorStrategy<RecordBatch>,
  error: &StreamError<RecordBatch>,
) -> ErrorAction {
  match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    ErrorStrategy::Skip => ErrorAction::Skip,
    ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}

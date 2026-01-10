//! Parquet write transformer for StreamWeave
//!
//! Writes data to Parquet files while passing data through. Takes RecordBatch as input, writes to Parquet,
//! and outputs the same data, enabling writing intermediate results while continuing processing.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use arrow::datatypes::SchemaRef;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tracing::{error, warn};

/// Configuration for Parquet writing behavior in transformers.
#[derive(Debug, Clone)]
pub struct ParquetWriteTransformerConfig {
  /// Compression codec to use.
  pub compression: ParquetWriteTransformerCompression,
  /// Maximum row group size.
  pub max_row_group_size: usize,
  /// Writer version.
  pub writer_version: ParquetWriteTransformerVersion,
}

/// Supported Parquet compression codecs for transformers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParquetWriteTransformerCompression {
  /// No compression.
  Uncompressed,
  /// Snappy compression.
  #[default]
  Snappy,
  /// LZ4 compression.
  Lz4,
  /// Zstd compression.
  Zstd,
}

impl From<ParquetWriteTransformerCompression> for Compression {
  fn from(compression: ParquetWriteTransformerCompression) -> Self {
    match compression {
      ParquetWriteTransformerCompression::Uncompressed => Compression::UNCOMPRESSED,
      ParquetWriteTransformerCompression::Snappy => Compression::SNAPPY,
      ParquetWriteTransformerCompression::Lz4 => Compression::LZ4,
      ParquetWriteTransformerCompression::Zstd => Compression::ZSTD(Default::default()),
    }
  }
}

/// Parquet writer version for transformers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParquetWriteTransformerVersion {
  /// Version 1.0.
  V1,
  /// Version 2.0.
  #[default]
  V2,
}

impl Default for ParquetWriteTransformerConfig {
  fn default() -> Self {
    Self {
      compression: ParquetWriteTransformerCompression::default(),
      max_row_group_size: 1024 * 1024,
      writer_version: ParquetWriteTransformerVersion::default(),
    }
  }
}

impl ParquetWriteTransformerConfig {
  /// Sets the compression codec.
  #[must_use]
  pub fn with_compression(mut self, compression: ParquetWriteTransformerCompression) -> Self {
    self.compression = compression;
    self
  }

  /// Sets the maximum row group size.
  #[must_use]
  pub fn with_max_row_group_size(mut self, size: usize) -> Self {
    self.max_row_group_size = size;
    self
  }

  /// Sets the writer version.
  #[must_use]
  pub fn with_writer_version(mut self, version: ParquetWriteTransformerVersion) -> Self {
    self.writer_version = version;
    self
  }

  /// Creates writer properties from this config.
  fn to_writer_properties(&self) -> WriterProperties {
    let mut builder = WriterProperties::builder()
      .set_compression(self.compression.into())
      .set_max_row_group_size(self.max_row_group_size);

    builder = match self.writer_version {
      ParquetWriteTransformerVersion::V1 => {
        builder.set_writer_version(parquet::file::properties::WriterVersion::PARQUET_1_0)
      }
      ParquetWriteTransformerVersion::V2 => {
        builder.set_writer_version(parquet::file::properties::WriterVersion::PARQUET_2_0)
      }
    };

    builder.build()
  }
}

/// A transformer that writes data to Parquet files while passing data through.
///
/// Each input RecordBatch is written to the Parquet file, and then the same batch is output,
/// enabling writing intermediate results while continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::ParquetWriteTransformer;
/// use arrow::record_batch::RecordBatch;
///
/// let transformer = ParquetWriteTransformer::new("output.parquet");
/// // Input: [RecordBatch, ...]
/// // Writes to Parquet and outputs: [RecordBatch, ...]
/// ```
pub struct ParquetWriteTransformer {
  /// Path to the Parquet file.
  path: PathBuf,
  /// Parquet-specific configuration.
  parquet_config: ParquetWriteTransformerConfig,
  /// Transformer configuration.
  config: TransformerConfig<RecordBatch>,
  /// Writer handle (shared across stream items).
  writer: Arc<Mutex<Option<ArrowWriter<File>>>>,
  /// Schema for the Parquet file (set from first batch).
  schema: Arc<Mutex<Option<SchemaRef>>>,
}

impl ParquetWriteTransformer {
  /// Creates a new `ParquetWriteTransformer` for the specified file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the Parquet file to write.
  #[must_use]
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      parquet_config: ParquetWriteTransformerConfig::default(),
      config: TransformerConfig::default(),
      writer: Arc::new(Mutex::new(None)),
      schema: Arc::new(Mutex::new(None)),
    }
  }

  /// Sets the error handling strategy for this transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<RecordBatch>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Sets the compression codec.
  #[must_use]
  pub fn with_compression(mut self, compression: ParquetWriteTransformerCompression) -> Self {
    self.parquet_config.compression = compression;
    self
  }

  /// Sets the maximum row group size.
  #[must_use]
  pub fn with_max_row_group_size(mut self, size: usize) -> Self {
    self.parquet_config.max_row_group_size = size;
    self
  }

  /// Sets the writer version.
  #[must_use]
  pub fn with_writer_version(mut self, version: ParquetWriteTransformerVersion) -> Self {
    self.parquet_config.writer_version = version;
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &PathBuf {
    &self.path
  }
}

impl Clone for ParquetWriteTransformer {
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      parquet_config: self.parquet_config.clone(),
      config: self.config.clone(),
      writer: Arc::clone(&self.writer),
      schema: Arc::clone(&self.schema),
    }
  }
}

impl Input for ParquetWriteTransformer {
  type Input = RecordBatch;
  type InputStream = Pin<Box<dyn Stream<Item = RecordBatch> + Send>>;
}

impl Output for ParquetWriteTransformer {
  type Output = RecordBatch;
  type OutputStream = Pin<Box<dyn Stream<Item = RecordBatch> + Send>>;
}

#[async_trait]
impl Transformer for ParquetWriteTransformer {
  type InputPorts = (RecordBatch,);
  type OutputPorts = (RecordBatch,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "parquet_write_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let parquet_config = self.parquet_config.clone();
    let writer = Arc::clone(&self.writer);
    let schema = Arc::clone(&self.schema);

    Box::pin(async_stream::stream! {
      let mut input_stream = input;
      while let Some(batch) = input_stream.next().await {
        // Initialize writer on first batch
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
                  "Failed to create Parquet file for writing"
                );
                return;
              }
            };

            let props = parquet_config.to_writer_properties();

            match ArrowWriter::try_new(file, batch.schema(), Some(props)) {
              Ok(w) => {
                *writer_guard = Some(w);
                *schema.lock().unwrap() = Some(batch.schema());
              }
              Err(e) => {
                error!(
                  component = %component_name,
                  path = %path.display(),
                  error = %e,
                  "Failed to create Parquet writer"
                );
                return;
              }
            }
          }
        }

        // Write the batch (synchronous operation, but we're in async context)
        {
          let mut writer_guard = writer.lock().unwrap();
          if let Some(ref mut w) = *writer_guard
            && let Err(e) = w.write(&batch) {
              let stream_error = StreamError::new(
                Box::new(e),
                ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: Some(batch.clone()),
                  component_name: component_name.clone(),
                  component_type: std::any::type_name::<ParquetWriteTransformer>().to_string(),
                },
                ComponentInfo {
                  name: component_name.clone(),
                  type_name: std::any::type_name::<ParquetWriteTransformer>().to_string(),
                },
              );

              match handle_error_strategy(&error_strategy, &stream_error) {
                ErrorAction::Stop => {
                  error!(
                    component = %component_name,
                    error = %stream_error,
                    "Stopping due to Parquet write error"
                  );
                  return;
                }
                ErrorAction::Skip => {
                  warn!(
                    component = %component_name,
                    error = %stream_error,
                    "Skipping batch due to Parquet write error"
                  );
                  continue;
                }
                ErrorAction::Retry => {
                  warn!(
                    component = %component_name,
                    error = %stream_error,
                    "Retry not supported for Parquet write errors, skipping"
                  );
                  continue;
                }
              }
            }
        }

        // Pass through the batch
        yield batch;
      }

      // Close the writer
      {
        let mut writer_guard = writer.lock().unwrap();
        if let Some(w) = writer_guard.take()
          && let Err(e) = w.close() {
            error!(
              component = %component_name,
              error = %e,
              "Failed to close Parquet writer"
            );
          }
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<RecordBatch>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<RecordBatch> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<RecordBatch> {
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
        .unwrap_or_else(|| "parquet_write_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "parquet_write_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

/// Helper function to handle error strategy
pub(crate) fn handle_error_strategy(
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

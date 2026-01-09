use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
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
use tracing::{error, warn};

/// Configuration for Parquet writing behavior.
#[derive(Debug, Clone)]
pub struct ParquetWriteConfig {
  /// Compression codec to use.
  pub compression: ParquetCompression,
  /// Maximum row group size.
  pub max_row_group_size: usize,
  /// Writer version.
  pub writer_version: ParquetWriterVersion,
}

/// Supported Parquet compression codecs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParquetCompression {
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

impl From<ParquetCompression> for Compression {
  fn from(compression: ParquetCompression) -> Self {
    match compression {
      ParquetCompression::Uncompressed => Compression::UNCOMPRESSED,
      ParquetCompression::Snappy => Compression::SNAPPY,
      ParquetCompression::Lz4 => Compression::LZ4,
      ParquetCompression::Zstd => Compression::ZSTD(Default::default()),
    }
  }
}

/// Parquet writer version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParquetWriterVersion {
  /// Version 1.0.
  V1,
  /// Version 2.0.
  #[default]
  V2,
}

impl Default for ParquetWriteConfig {
  fn default() -> Self {
    Self {
      compression: ParquetCompression::default(),
      max_row_group_size: 1024 * 1024,
      writer_version: ParquetWriterVersion::default(),
    }
  }
}

impl ParquetWriteConfig {
  /// Sets the compression codec.
  #[must_use]
  pub fn with_compression(mut self, compression: ParquetCompression) -> Self {
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
  pub fn with_writer_version(mut self, version: ParquetWriterVersion) -> Self {
    self.writer_version = version;
    self
  }

  /// Creates writer properties from this config.
  pub(crate) fn to_writer_properties(&self) -> WriterProperties {
    let mut builder = WriterProperties::builder()
      .set_compression(self.compression.into())
      .set_max_row_group_size(self.max_row_group_size);

    builder = match self.writer_version {
      ParquetWriterVersion::V1 => {
        builder.set_writer_version(parquet::file::properties::WriterVersion::PARQUET_1_0)
      }
      ParquetWriterVersion::V2 => {
        builder.set_writer_version(parquet::file::properties::WriterVersion::PARQUET_2_0)
      }
    };

    builder.build()
  }
}

/// A consumer that writes Apache Parquet files from Arrow RecordBatches.
///
/// Parquet is a columnar storage format that's efficient for analytics workloads.
/// This consumer collects Arrow RecordBatch objects and writes them to a Parquet file.
///
/// # Example
///
/// ```ignore
/// use crate::consumers::ParquetConsumer;
///
/// let consumer = ParquetConsumer::new("output.parquet");
/// ```
pub struct ParquetConsumer {
  /// Path to the Parquet file.
  pub path: PathBuf,
  /// Consumer configuration.
  pub config: ConsumerConfig<RecordBatch>,
  /// Parquet-specific configuration.
  pub parquet_config: ParquetWriteConfig,
  /// Schema for the Parquet file (set from first batch).
  pub schema: Option<SchemaRef>,
  /// Writer handle (opened on first write).
  pub writer: Option<ArrowWriter<File>>,
}

impl ParquetConsumer {
  /// Creates a new Parquet consumer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      config: ConsumerConfig::default(),
      parquet_config: ParquetWriteConfig::default(),
      schema: None,
      writer: None,
    }
  }

  /// Sets the error strategy for the consumer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<RecordBatch>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the consumer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  /// Sets the compression codec.
  #[must_use]
  pub fn with_compression(mut self, compression: ParquetCompression) -> Self {
    self.parquet_config.compression = compression;
    self
  }

  /// Sets the maximum row group size.
  #[must_use]
  pub fn with_max_row_group_size(mut self, size: usize) -> Self {
    self.parquet_config.max_row_group_size = size;
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &PathBuf {
    &self.path
  }
}

// Trait implementations for ParquetConsumer

impl Input for ParquetConsumer {
  type Input = RecordBatch;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

#[async_trait]
impl Consumer for ParquetConsumer {
  type InputPorts = (RecordBatch,);

  /// Consumes a stream of Arrow RecordBatches and writes them to a Parquet file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be created, an error is logged and no data is written.
  /// - If a batch cannot be written, the error strategy determines the action.
  async fn consume(&mut self, input: Self::InputStream) {
    let path = self.path.clone();
    let component_name = if self.config.name.is_empty() {
      "parquet_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let error_strategy = self.config.error_strategy.clone();
    let parquet_config = self.parquet_config.clone();

    let mut input = std::pin::pin!(input);
    let mut writer: Option<ArrowWriter<File>> = None;

    while let Some(batch) = input.next().await {
      // Initialize writer on first batch
      if writer.is_none() {
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
          Ok(w) => writer = Some(w),
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

      // Write the batch
      if let Some(ref mut w) = writer
        && let Err(e) = w.write(&batch)
      {
        let stream_error = StreamError::new(
          Box::new(e),
          ErrorContext {
            timestamp: chrono::Utc::now(),
            item: Some(batch.clone()),
            component_name: component_name.clone(),
            component_type: std::any::type_name::<Self>().to_string(),
          },
          ComponentInfo {
            name: component_name.clone(),
            type_name: std::any::type_name::<Self>().to_string(),
          },
        );

        match handle_error_strategy(&error_strategy, &stream_error) {
          ErrorAction::Stop => {
            error!(
              component = %component_name,
              error = %stream_error,
              "Stopping due to write error"
            );
            break;
          }
          ErrorAction::Skip => {
            warn!(
              component = %component_name,
              error = %stream_error,
              "Skipping batch due to write error"
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

    // Close the writer
    if let Some(w) = writer
      && let Err(e) = w.close()
    {
      error!(
        component = %component_name,
        error = %e,
        "Failed to close Parquet writer"
      );
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<RecordBatch>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<RecordBatch> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<RecordBatch> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<RecordBatch>) -> ErrorAction {
    match self.config.error_strategy {
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

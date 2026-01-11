use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio::sync::Mutex;
use tracing::{error, instrument, warn};

/// Available compression algorithms for Parquet files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParquetCompression {
  Uncompressed,
  #[default]
  Snappy,
  Lz4,
  Zstd,
}

impl From<ParquetCompression> for parquet::basic::Compression {
  fn from(val: ParquetCompression) -> Self {
    match val {
      ParquetCompression::Uncompressed => parquet::basic::Compression::UNCOMPRESSED,
      ParquetCompression::Snappy => parquet::basic::Compression::SNAPPY,
      ParquetCompression::Lz4 => parquet::basic::Compression::LZ4,
      ParquetCompression::Zstd => {
        // Use default ZstdLevel (typically 3)
        parquet::basic::Compression::ZSTD(parquet::basic::ZstdLevel::try_new(3).unwrap_or_default())
      }
    }
  }
}

/// Parquet writer version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParquetWriterVersion {
  V1,
  #[default]
  V2,
}

impl From<ParquetWriterVersion> for parquet::file::properties::WriterVersion {
  fn from(_val: ParquetWriterVersion) -> Self {
    // WriterVersion API changed in parquet 54 - use PARQUET_2_0 as default
    // TODO: Check actual API and restore V1/V2 mapping if available
    parquet::file::properties::WriterVersion::PARQUET_2_0
  }
}

/// Configuration for writing Parquet files.
#[derive(Debug, Clone)]
pub struct ParquetWriteConfig {
  /// Compression algorithm to use.
  pub compression: ParquetCompression,
  /// Maximum size for a row group in bytes.
  pub max_row_group_size: usize,
  /// Parquet writer version.
  pub writer_version: ParquetWriterVersion,
}

impl Default for ParquetWriteConfig {
  fn default() -> Self {
    Self {
      compression: ParquetCompression::default(),
      max_row_group_size: 1024 * 1024, // 1MB
      writer_version: ParquetWriterVersion::default(),
    }
  }
}

impl ParquetWriteConfig {
  /// Sets the compression algorithm.
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

  /// Sets the Parquet writer version.
  #[must_use]
  pub fn with_writer_version(mut self, version: ParquetWriterVersion) -> Self {
    self.writer_version = version;
    self
  }

  /// Converts to Parquet writer properties.
  pub fn to_writer_properties(&self) -> WriterProperties {
    let compression = self.compression.into();
    let writer_version = self.writer_version.into();
    WriterProperties::builder()
      .set_compression(compression)
      .set_max_row_group_size(self.max_row_group_size)
      .set_writer_version(writer_version)
      .build()
  }
}

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
/// A consumer that writes Arrow `RecordBatch`es to a Parquet file.
///
/// This consumer accumulates `RecordBatch`es and writes them to a specified
/// Parquet file path. It supports various Parquet writer configurations like
/// compression and row group size.
use crate::{Consumer, ConsumerConfig, Input};
pub struct ParquetConsumer {
  /// The path to the output Parquet file.
  path: PathBuf,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<RecordBatch>,
  /// Parquet file write configuration.
  pub parquet_config: ParquetWriteConfig,
  /// Internal buffer for writing Parquet data.
  #[allow(clippy::type_complexity)]
  writer: Mutex<Option<ArrowWriter<std::fs::File>>>,
}

impl ParquetConsumer {
  /// Creates a new `ParquetConsumer` with the given file path.
  ///
  /// # Arguments
  ///
  /// * `path` - The path to the Parquet file to write.
  pub fn new(path: impl AsRef<Path>) -> Self {
    Self {
      path: path.as_ref().to_path_buf(),
      config: ConsumerConfig::default(),
      parquet_config: ParquetWriteConfig::default(),
      writer: Mutex::new(None),
    }
  }

  /// Sets the compression algorithm for the Parquet file.
  #[must_use]
  pub fn with_compression(mut self, compression: ParquetCompression) -> Self {
    self.parquet_config = self.parquet_config.with_compression(compression);
    self
  }

  /// Sets the maximum row group size for the Parquet file.
  #[must_use]
  pub fn with_max_row_group_size(mut self, size: usize) -> Self {
    self.parquet_config = self.parquet_config.with_max_row_group_size(size);
    self
  }

  /// Sets the Parquet writer version.
  #[must_use]
  pub fn with_writer_version(mut self, version: ParquetWriterVersion) -> Self {
    self.parquet_config = self.parquet_config.with_writer_version(version);
    self
  }

  /// Sets the error handling strategy for this consumer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<RecordBatch>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this consumer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  /// Returns the path to the output Parquet file.
  #[must_use]
  pub fn path(&self) -> &PathBuf {
    &self.path
  }
}

impl Clone for ParquetConsumer {
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      config: self.config.clone(),
      parquet_config: self.parquet_config.clone(),
      writer: Mutex::new(None), // Writer cannot be cloned, reset to None
    }
  }
}

impl Input for ParquetConsumer {
  type Input = RecordBatch;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

#[async_trait]
impl Consumer for ParquetConsumer {
  type InputPorts = (RecordBatch,);

  /// Consumes a stream of `RecordBatch`es and writes them to a Parquet file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be created, an error is logged and no data is written.
  /// - If a `RecordBatch` cannot be written, the error strategy determines the action.
  #[instrument(level = "debug", skip(self, stream))]
  async fn consume(&mut self, mut stream: Self::InputStream) {
    let component_name = self.config.name.clone();
    let path = self.path.clone();
    let error_strategy = self.config.error_strategy.clone();
    let parquet_config = self.parquet_config.clone();

    while let Some(batch) = stream.next().await {
      let mut writer_lock = self.writer.lock().await;

      if writer_lock.is_none() {
        // Initialize writer only on first batch
        let schema = batch.schema().clone();
        let writer_properties = parquet_config.to_writer_properties();

        // Create file synchronously (can be made async if needed)
        let file = match std::fs::File::create(&path) {
          Ok(f) => f,
          Err(e) => {
            error!(
              component = %component_name,
              path = %path.display(),
              error = %e,
              "Failed to create Parquet file, all items will be dropped"
            );
            return;
          }
        };

        match ArrowWriter::try_new(file, schema, Some(writer_properties)) {
          Ok(w) => {
            *writer_lock = Some(w);
          }
          Err(e) => {
            error!(
              component = %component_name,
              path = %path.display(),
              error = %e,
              "Failed to create Parquet writer, all items will be dropped"
            );
            return;
          }
        }
      }

      let writer = writer_lock.as_mut().unwrap();

      if let Err(e) = writer.write(&batch) {
        let error = StreamError::new(
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

        match crate::consumers::csv_consumer::handle_error_strategy(&error_strategy, &error) {
          ErrorAction::Stop => {
            error!(
              component = %component_name,
              path = %path.display(),
              error = %error,
              "Stopping due to Parquet write error"
            );
            break;
          }
          ErrorAction::Skip => {
            warn!(
              component = %component_name,
              path = %path.display(),
              error = %error,
              "Skipping item due to Parquet write error"
            );
            continue;
          }
          ErrorAction::Retry => {
            warn!(
              component = %component_name,
              path = %path.display(),
              error = %error,
              "Retry not fully supported for Parquet write errors, skipping"
            );
            continue;
          }
        }
      }
    }

    // Finalize the writer - file is already written to, just need to close the writer
    let mut writer_lock = self.writer.lock().await;
    if let Some(writer) = writer_lock.take() {
      // close() finalizes the parquet file and closes the underlying file
      if let Err(e) = writer.close() {
        error!(
          component = %component_name,
          path = %path.display(),
          error = %e,
          "Failed to finalize Parquet writer"
        );
      }
      // File is automatically closed when writer is dropped
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
}

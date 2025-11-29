use crate::consumer::ConsumerConfig;
use crate::error::ErrorStrategy;
use arrow::datatypes::SchemaRef;
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::path::PathBuf;

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
/// use streamweave::consumers::parquet::parquet_consumer::ParquetConsumer;
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

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use proptest::proptest;

  proptest! {
    #[test]
    fn test_parquet_consumer_new(path in "[a-zA-Z0-9_./-]+\\.parquet") {
      let consumer = ParquetConsumer::new(path.clone());
      prop_assert_eq!(consumer.path(), &PathBuf::from(path));
    }

    #[test]
    fn test_parquet_consumer_builder(
      path in "[a-zA-Z0-9_./-]+\\.parquet",
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
      compression in prop::sample::select(vec![
        ParquetCompression::Uncompressed,
        ParquetCompression::Snappy,
        ParquetCompression::Lz4,
        ParquetCompression::Zstd,
      ]),
      max_row_group_size in 1024usize..1048576usize
    ) {
      let consumer = ParquetConsumer::new(path.clone())
        .with_name(name.clone())
        .with_compression(compression)
        .with_max_row_group_size(max_row_group_size);

      prop_assert_eq!(consumer.path(), &PathBuf::from(path));
      prop_assert_eq!(consumer.config.name, name);
      prop_assert_eq!(consumer.parquet_config.compression, compression);
      prop_assert_eq!(consumer.parquet_config.max_row_group_size, max_row_group_size);
    }

    #[test]
    fn test_parquet_write_config_default(_ in prop::num::u8::ANY) {
      let config = ParquetWriteConfig::default();
      prop_assert_eq!(config.compression, ParquetCompression::Snappy);
      prop_assert_eq!(config.max_row_group_size, 1024 * 1024);
      prop_assert_eq!(config.writer_version, ParquetWriterVersion::V2);
    }

    #[test]
    fn test_parquet_compression_conversion(
      compression in prop::sample::select(vec![
        ParquetCompression::Uncompressed,
        ParquetCompression::Snappy,
        ParquetCompression::Lz4,
        ParquetCompression::Zstd,
      ])
    ) {
      let result: Compression = compression.into();
      match compression {
        ParquetCompression::Uncompressed => {
          prop_assert!(matches!(result, Compression::UNCOMPRESSED));
        }
        ParquetCompression::Snappy => {
          prop_assert!(matches!(result, Compression::SNAPPY));
        }
        ParquetCompression::Lz4 => {
          prop_assert!(matches!(result, Compression::LZ4));
        }
        ParquetCompression::Zstd => {
          prop_assert!(matches!(result, Compression::ZSTD(_)));
        }
      }
    }
  }
}

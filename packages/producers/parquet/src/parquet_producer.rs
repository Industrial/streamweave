use arrow::record_batch::RecordBatch;
use streamweave_core::ProducerConfig;
use streamweave_error::ErrorStrategy;

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
/// use streamweave::producers::parquet::parquet_producer::ParquetProducer;
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parquet_producer_new() {
    let producer = ParquetProducer::new("test.parquet");
    assert_eq!(producer.path(), "test.parquet");
  }

  #[test]
  fn test_parquet_producer_builder() {
    let producer = ParquetProducer::new("test.parquet")
      .with_name("test_producer".to_string())
      .with_batch_size(2048)
      .with_projection(vec![0, 1, 2]);

    assert_eq!(producer.path(), "test.parquet");
    assert_eq!(producer.config.name, Some("test_producer".to_string()));
    assert_eq!(producer.parquet_config.batch_size, 2048);
    assert_eq!(producer.parquet_config.projection, Some(vec![0, 1, 2]));
  }

  #[test]
  fn test_parquet_read_config_default() {
    let config = ParquetReadConfig::default();
    assert_eq!(config.batch_size, 1024);
    assert_eq!(config.projection, None);
    assert_eq!(config.row_groups, None);
  }
}

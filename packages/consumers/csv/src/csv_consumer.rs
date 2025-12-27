use csv::Writer;
use serde::Serialize;
use std::fs::File;
use std::marker::PhantomData;
use std::path::PathBuf;
use streamweave_core::ConsumerConfig;
use streamweave_error::ErrorStrategy;

/// Configuration for CSV writing behavior.
#[derive(Debug, Clone)]
pub struct CsvWriteConfig {
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

impl Default for CsvWriteConfig {
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

impl CsvWriteConfig {
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

/// A consumer that writes CSV files.
///
/// # Example
///
/// ```ignore
/// use streamweave::consumers::csv::csv_consumer::CsvConsumer;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Record {
///     name: String,
///     age: u32,
/// }
///
/// let consumer = CsvConsumer::<Record>::new("output.csv");
/// ```
pub struct CsvConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Path to the CSV file.
  pub path: PathBuf,
  /// Consumer configuration.
  pub config: ConsumerConfig<T>,
  /// CSV-specific configuration.
  pub csv_config: CsvWriteConfig,
  /// Writer handle (opened on first write).
  pub writer: Option<Writer<File>>,
  /// Whether we've written the first record (for header handling).
  pub first_record_written: bool,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> CsvConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new CSV consumer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      config: ConsumerConfig::default(),
      csv_config: CsvWriteConfig::default(),
      writer: None,
      first_record_written: false,
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

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use proptest::proptest;
  use serde::Serialize;

  #[derive(Debug, Clone, Serialize)]
  struct TestRecord {
    name: String,
    age: u32,
  }

  proptest! {
    #[test]
    fn test_csv_consumer_new(path in "[a-zA-Z0-9_./-]+\\.csv") {
      let consumer = CsvConsumer::<TestRecord>::new(path.clone());
      prop_assert_eq!(consumer.path(), &PathBuf::from(path));
    }

    #[test]
    fn test_csv_consumer_builder(
      path in "[a-zA-Z0-9_./-]+\\.csv",
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
      write_headers in prop::bool::ANY,
      delimiter in prop::num::u8::ANY,
      flush_on_write in prop::bool::ANY
    ) {
      let consumer = CsvConsumer::<TestRecord>::new(path.clone())
        .with_name(name.clone())
        .with_headers(write_headers)
        .with_delimiter(delimiter)
        .with_flush_on_write(flush_on_write);

      prop_assert_eq!(consumer.path(), &PathBuf::from(path));
      prop_assert_eq!(consumer.config.name, name);
      prop_assert_eq!(consumer.csv_config.write_headers, write_headers);
      prop_assert_eq!(consumer.csv_config.delimiter, delimiter);
      prop_assert_eq!(consumer.csv_config.flush_on_write, flush_on_write);
    }

    #[test]
    fn test_csv_write_config_default(_ in prop::num::u8::ANY) {
      let config = CsvWriteConfig::default();
      prop_assert!(config.write_headers);
      prop_assert_eq!(config.delimiter, b',');
      prop_assert_eq!(config.quote, b'"');
      prop_assert!(config.double_quote);
      prop_assert!(!config.flush_on_write);
    }

    #[test]
    fn test_csv_write_config_with_headers(
      write_headers in prop::bool::ANY
    ) {
      let config = CsvWriteConfig::default().with_headers(write_headers);
      prop_assert_eq!(config.write_headers, write_headers);
    }

    #[test]
    fn test_csv_write_config_with_delimiter(
      delimiter in prop::num::u8::ANY
    ) {
      let config = CsvWriteConfig::default().with_delimiter(delimiter);
      prop_assert_eq!(config.delimiter, delimiter);
    }

    #[test]
    fn test_csv_write_config_with_flush_on_write(
      flush in prop::bool::ANY
    ) {
      let config = CsvWriteConfig::default().with_flush_on_write(flush);
      prop_assert_eq!(config.flush_on_write, flush);
    }

    #[test]
    fn test_csv_write_config_chaining(
      write_headers in prop::bool::ANY,
      delimiter in prop::num::u8::ANY,
      flush in prop::bool::ANY
    ) {
      let config = CsvWriteConfig::default()
        .with_headers(write_headers)
        .with_delimiter(delimiter)
        .with_flush_on_write(flush);
      prop_assert_eq!(config.write_headers, write_headers);
      prop_assert_eq!(config.delimiter, delimiter);
      prop_assert_eq!(config.flush_on_write, flush);
    }
  }
}

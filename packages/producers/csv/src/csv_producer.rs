use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use streamweave_core::ProducerConfig;
use streamweave_error::ErrorStrategy;

/// Configuration for CSV reading behavior.
#[derive(Debug, Clone)]
pub struct CsvReadConfig {
  /// Whether the CSV has a header row.
  pub has_headers: bool,
  /// The delimiter character (default: comma).
  pub delimiter: u8,
  /// Whether to allow flexible column counts.
  pub flexible: bool,
  /// Whether to trim whitespace from fields.
  pub trim: bool,
  /// The comment character (None means no comments).
  pub comment: Option<u8>,
  /// The quote character (default: double quote).
  pub quote: u8,
  /// Whether double quotes are used for escaping.
  pub double_quote: bool,
}

impl Default for CsvReadConfig {
  fn default() -> Self {
    Self {
      has_headers: true,
      delimiter: b',',
      flexible: false,
      trim: false,
      comment: None,
      quote: b'"',
      double_quote: true,
    }
  }
}

impl CsvReadConfig {
  /// Sets whether the CSV has a header row.
  #[must_use]
  pub fn with_headers(mut self, has_headers: bool) -> Self {
    self.has_headers = has_headers;
    self
  }

  /// Sets the delimiter character.
  #[must_use]
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.delimiter = delimiter;
    self
  }

  /// Sets whether to allow flexible column counts.
  #[must_use]
  pub fn with_flexible(mut self, flexible: bool) -> Self {
    self.flexible = flexible;
    self
  }

  /// Sets whether to trim whitespace from fields.
  #[must_use]
  pub fn with_trim(mut self, trim: bool) -> Self {
    self.trim = trim;
    self
  }

  /// Sets the comment character.
  #[must_use]
  pub fn with_comment(mut self, comment: Option<u8>) -> Self {
    self.comment = comment;
    self
  }
}

/// A producer that reads CSV files and deserializes rows.
///
/// # Example
///
/// ```ignore
/// use streamweave::producers::csv::csv_producer::CsvProducer;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Record {
///     name: String,
///     age: u32,
/// }
///
/// let producer = CsvProducer::<Record>::new("data.csv");
/// ```
pub struct CsvProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Path to the CSV file.
  pub path: String,
  /// Producer configuration.
  pub config: ProducerConfig<T>,
  /// CSV-specific configuration.
  pub csv_config: CsvReadConfig,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> CsvProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Creates a new CSV producer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<String>) -> Self {
    Self {
      path: path.into(),
      config: ProducerConfig::default(),
      csv_config: CsvReadConfig::default(),
      _phantom: PhantomData,
    }
  }

  /// Sets the error strategy for the producer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the producer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Sets whether the CSV has a header row.
  #[must_use]
  pub fn with_headers(mut self, has_headers: bool) -> Self {
    self.csv_config.has_headers = has_headers;
    self
  }

  /// Sets the delimiter character.
  #[must_use]
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.csv_config.delimiter = delimiter;
    self
  }

  /// Sets whether to allow flexible column counts.
  #[must_use]
  pub fn with_flexible(mut self, flexible: bool) -> Self {
    self.csv_config.flexible = flexible;
    self
  }

  /// Sets whether to trim whitespace from fields.
  #[must_use]
  pub fn with_trim(mut self, trim: bool) -> Self {
    self.csv_config.trim = trim;
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &str {
    &self.path
  }
}

impl<T> Clone for CsvProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      config: self.config.clone(),
      csv_config: self.csv_config.clone(),
      _phantom: PhantomData,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde::Deserialize;

  #[derive(Debug, Clone, Deserialize)]
  struct TestRecord {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    age: u32,
  }

  #[test]
  fn test_csv_producer_new() {
    let producer = CsvProducer::<TestRecord>::new("test.csv");
    assert_eq!(producer.path(), "test.csv");
  }

  #[test]
  fn test_csv_producer_builder() {
    let producer = CsvProducer::<TestRecord>::new("test.csv")
      .with_name("test_producer".to_string())
      .with_headers(false)
      .with_delimiter(b'\t')
      .with_flexible(true)
      .with_trim(true);

    assert_eq!(producer.path(), "test.csv");
    assert_eq!(producer.config.name, Some("test_producer".to_string()));
    assert!(!producer.csv_config.has_headers);
    assert_eq!(producer.csv_config.delimiter, b'\t');
    assert!(producer.csv_config.flexible);
    assert!(producer.csv_config.trim);
  }

  #[test]
  fn test_csv_read_config_default() {
    let config = CsvReadConfig::default();
    assert!(config.has_headers);
    assert_eq!(config.delimiter, b',');
    assert!(!config.flexible);
    assert!(!config.trim);
    assert_eq!(config.comment, None);
    assert_eq!(config.quote, b'"');
    assert!(config.double_quote);
  }
}

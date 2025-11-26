use crate::consumer::ConsumerConfig;
use crate::error::ErrorStrategy;
use serde::Serialize;
use std::marker::PhantomData;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::BufWriter;

/// Configuration for JSONL writing behavior.
#[derive(Debug, Clone)]
pub struct JsonlWriteConfig {
  /// Whether to append to existing file or overwrite.
  pub append: bool,
  /// Whether to pretty-print JSON (not recommended for JSONL).
  pub pretty: bool,
  /// Buffer size for writing.
  pub buffer_size: usize,
}

impl Default for JsonlWriteConfig {
  fn default() -> Self {
    Self {
      append: false,
      pretty: false,
      buffer_size: 8192,
    }
  }
}

impl JsonlWriteConfig {
  /// Sets whether to append to existing file.
  #[must_use]
  pub fn with_append(mut self, append: bool) -> Self {
    self.append = append;
    self
  }

  /// Sets the buffer size.
  #[must_use]
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.buffer_size = size;
    self
  }
}

/// A consumer that writes JSON Lines (JSONL) files.
///
/// JSON Lines is a text format where each line is a valid JSON value.
/// This consumer serializes each input element as JSON and writes it
/// as a single line to the output file.
///
/// # Example
///
/// ```ignore
/// use streamweave::consumers::jsonl::jsonl_consumer::JsonlConsumer;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let consumer = JsonlConsumer::<Event>::new("events.jsonl");
/// ```
pub struct JsonlConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Path to the JSONL file.
  pub path: PathBuf,
  /// Consumer configuration.
  pub config: ConsumerConfig<T>,
  /// JSONL-specific configuration.
  pub jsonl_config: JsonlWriteConfig,
  /// File handle (opened on first write).
  pub file: Option<BufWriter<File>>,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> JsonlConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new JSONL consumer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      config: ConsumerConfig::default(),
      jsonl_config: JsonlWriteConfig::default(),
      file: None,
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

  /// Sets whether to append to existing file.
  #[must_use]
  pub fn with_append(mut self, append: bool) -> Self {
    self.jsonl_config.append = append;
    self
  }

  /// Sets the buffer size.
  #[must_use]
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.jsonl_config.buffer_size = size;
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
  use serde::Serialize;

  #[derive(Debug, Clone, Serialize)]
  struct TestRecord {
    id: u32,
    name: String,
  }

  #[test]
  fn test_jsonl_consumer_new() {
    let consumer = JsonlConsumer::<TestRecord>::new("test.jsonl");
    assert_eq!(consumer.path(), &PathBuf::from("test.jsonl"));
  }

  #[test]
  fn test_jsonl_consumer_builder() {
    let consumer = JsonlConsumer::<TestRecord>::new("test.jsonl")
      .with_name("test_consumer".to_string())
      .with_append(true)
      .with_buffer_size(16384);

    assert_eq!(consumer.path(), &PathBuf::from("test.jsonl"));
    assert_eq!(consumer.config.name, "test_consumer");
    assert!(consumer.jsonl_config.append);
    assert_eq!(consumer.jsonl_config.buffer_size, 16384);
  }

  #[test]
  fn test_jsonl_write_config_default() {
    let config = JsonlWriteConfig::default();
    assert!(!config.append);
    assert!(!config.pretty);
    assert_eq!(config.buffer_size, 8192);
  }

  #[test]
  fn test_jsonl_write_config_builder() {
    let config = JsonlWriteConfig::default()
      .with_append(true)
      .with_buffer_size(4096);

    assert!(config.append);
    assert_eq!(config.buffer_size, 4096);
  }
}

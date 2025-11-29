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
  use proptest::prelude::*;
  use proptest::proptest;
  use serde::Serialize;

  #[derive(Debug, Clone, Serialize)]
  struct TestRecord {
    id: u32,
    name: String,
  }

  proptest! {
    #[test]
    fn test_jsonl_consumer_new(path in "[a-zA-Z0-9_./-]+\\.jsonl") {
      let consumer = JsonlConsumer::<TestRecord>::new(path.clone());
      prop_assert_eq!(consumer.path(), &PathBuf::from(path));
    }

    #[test]
    fn test_jsonl_consumer_builder(
      path in "[a-zA-Z0-9_./-]+\\.jsonl",
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
      append in prop::bool::ANY,
      buffer_size in 1024usize..65536usize
    ) {
      let consumer = JsonlConsumer::<TestRecord>::new(path.clone())
        .with_name(name.clone())
        .with_append(append)
        .with_buffer_size(buffer_size);

      prop_assert_eq!(consumer.path(), &PathBuf::from(path));
      prop_assert_eq!(consumer.config.name, name);
      prop_assert_eq!(consumer.jsonl_config.append, append);
      prop_assert_eq!(consumer.jsonl_config.buffer_size, buffer_size);
    }

    #[test]
    fn test_jsonl_write_config_default(_ in prop::num::u8::ANY) {
      let config = JsonlWriteConfig::default();
      prop_assert!(!config.append);
      prop_assert!(!config.pretty);
      prop_assert_eq!(config.buffer_size, 8192);
    }

    #[test]
    fn test_jsonl_write_config_builder(
      append in prop::bool::ANY,
      buffer_size in 1024usize..65536usize
    ) {
      let config = JsonlWriteConfig::default()
        .with_append(append)
        .with_buffer_size(buffer_size);

      prop_assert_eq!(config.append, append);
      prop_assert_eq!(config.buffer_size, buffer_size);
    }
  }
}

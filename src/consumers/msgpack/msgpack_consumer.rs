use crate::consumer::ConsumerConfig;
use crate::error::ErrorStrategy;
use serde::Serialize;
use std::fs::File;
use std::io::BufWriter;
use std::marker::PhantomData;
use std::path::PathBuf;

/// Configuration for MessagePack writing behavior.
#[derive(Debug, Clone)]
pub struct MsgPackWriteConfig {
  /// Whether to use named struct fields (as a map) or positional (as an array).
  /// When true, structs are written as MessagePack maps; when false, as arrays.
  pub named_fields: bool,
}

impl Default for MsgPackWriteConfig {
  fn default() -> Self {
    Self { named_fields: true }
  }
}

impl MsgPackWriteConfig {
  /// Sets whether to use named struct fields.
  #[must_use]
  pub fn with_named_fields(mut self, named: bool) -> Self {
    self.named_fields = named;
    self
  }
}

/// A consumer that writes MessagePack files.
///
/// MessagePack is a binary serialization format that's more compact than JSON
/// but supports similar data types. This consumer serializes each input element
/// and writes it to the output file.
///
/// # Example
///
/// ```ignore
/// use streamweave::consumers::msgpack::msgpack_consumer::MsgPackConsumer;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let consumer = MsgPackConsumer::<Event>::new("events.msgpack");
/// ```
pub struct MsgPackConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Path to the MessagePack file.
  pub path: PathBuf,
  /// Consumer configuration.
  pub config: ConsumerConfig<T>,
  /// MessagePack-specific configuration.
  pub msgpack_config: MsgPackWriteConfig,
  /// Writer handle (opened on first write).
  pub writer: Option<BufWriter<File>>,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> MsgPackConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new MessagePack consumer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      config: ConsumerConfig::default(),
      msgpack_config: MsgPackWriteConfig::default(),
      writer: None,
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
    name: String,
    age: u32,
  }

  #[test]
  fn test_msgpack_consumer_new() {
    let consumer = MsgPackConsumer::<TestRecord>::new("test.msgpack");
    assert_eq!(consumer.path(), &PathBuf::from("test.msgpack"));
  }

  #[test]
  fn test_msgpack_consumer_builder() {
    let consumer = MsgPackConsumer::<TestRecord>::new("test.msgpack")
      .with_name("test_consumer".to_string())
      .with_error_strategy(ErrorStrategy::Skip);

    assert_eq!(consumer.path(), &PathBuf::from("test.msgpack"));
    assert_eq!(consumer.config.name, "test_consumer");
    assert!(matches!(
      consumer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_msgpack_write_config_default() {
    let config = MsgPackWriteConfig::default();
    assert!(config.named_fields);
  }
}

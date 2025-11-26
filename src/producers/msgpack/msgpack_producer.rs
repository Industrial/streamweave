use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

/// Configuration for MessagePack reading behavior.
#[derive(Debug, Clone)]
pub struct MsgPackReadConfig {
  /// Whether to use named struct fields (as a map) or positional (as an array).
  /// When true, structs are read as MessagePack maps; when false, as arrays.
  pub named_fields: bool,
}

impl Default for MsgPackReadConfig {
  fn default() -> Self {
    Self { named_fields: true }
  }
}

impl MsgPackReadConfig {
  /// Sets whether to use named struct fields.
  #[must_use]
  pub fn with_named_fields(mut self, named: bool) -> Self {
    self.named_fields = named;
    self
  }
}

/// A producer that reads MessagePack files.
///
/// MessagePack is a binary serialization format that's more compact than JSON
/// but supports similar data types. This producer reads serialized objects
/// from a file and yields them as deserialized values.
///
/// # Example
///
/// ```ignore
/// use streamweave::producers::msgpack::msgpack_producer::MsgPackProducer;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let producer = MsgPackProducer::<Event>::new("events.msgpack");
/// ```
pub struct MsgPackProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Path to the MessagePack file.
  pub path: String,
  /// Producer configuration.
  pub config: ProducerConfig<T>,
  /// MessagePack-specific configuration.
  pub msgpack_config: MsgPackReadConfig,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> MsgPackProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Creates a new MessagePack producer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<String>) -> Self {
    Self {
      path: path.into(),
      config: ProducerConfig::default(),
      msgpack_config: MsgPackReadConfig::default(),
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

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &str {
    &self.path
  }
}

impl<T> Clone for MsgPackProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      config: self.config.clone(),
      msgpack_config: self.msgpack_config.clone(),
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
  fn test_msgpack_producer_new() {
    let producer = MsgPackProducer::<TestRecord>::new("test.msgpack");
    assert_eq!(producer.path(), "test.msgpack");
  }

  #[test]
  fn test_msgpack_producer_builder() {
    let producer = MsgPackProducer::<TestRecord>::new("test.msgpack")
      .with_name("test_producer".to_string())
      .with_error_strategy(ErrorStrategy::Skip);

    assert_eq!(producer.path(), "test.msgpack");
    assert_eq!(producer.config.name, Some("test_producer".to_string()));
    assert!(matches!(
      producer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_msgpack_read_config_default() {
    let config = MsgPackReadConfig::default();
    assert!(config.named_fields);
  }
}

use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

/// A producer that reads JSON Lines (JSONL) files.
///
/// JSON Lines is a text format where each line is a valid JSON value.
/// This producer reads each line from a file and deserializes it as
/// the specified type.
///
/// # Example
///
/// ```ignore
/// use streamweave::producers::jsonl::jsonl_producer::JsonlProducer;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let producer = JsonlProducer::<Event>::new("events.jsonl");
/// ```
pub struct JsonlProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// The path to the JSONL file to read from.
  pub path: String,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> JsonlProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Creates a new JSONL producer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<String>) -> Self {
    Self {
      path: path.into(),
      config: ProducerConfig::default(),
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

impl<T> Clone for JsonlProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      config: self.config.clone(),
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
    id: u32,
    #[allow(dead_code)]
    name: String,
  }

  #[test]
  fn test_jsonl_producer_new() {
    let producer = JsonlProducer::<TestRecord>::new("test.jsonl");
    assert_eq!(producer.path(), "test.jsonl");
  }

  #[test]
  fn test_jsonl_producer_builder() {
    let producer = JsonlProducer::<TestRecord>::new("test.jsonl")
      .with_name("test_producer".to_string())
      .with_error_strategy(ErrorStrategy::Skip);

    assert_eq!(producer.path(), "test.jsonl");
    assert_eq!(producer.config.name, Some("test_producer".to_string()));
    assert!(matches!(
      producer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_jsonl_producer_clone() {
    let producer =
      JsonlProducer::<TestRecord>::new("test.jsonl").with_name("test_producer".to_string());

    let cloned = producer.clone();
    assert_eq!(cloned.path(), producer.path());
    assert_eq!(cloned.config.name, producer.config.name);
  }
}

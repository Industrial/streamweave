use crate::consumer::ConsumerConfig;
use crate::error::ErrorStrategy;
use serde::Serialize;
use std::collections::HashMap;

/// Configuration for Kafka producer behavior.
#[derive(Debug, Clone)]
pub struct KafkaProducerConfig {
  /// Bootstrap servers (comma-separated list of broker addresses).
  pub bootstrap_servers: String,
  /// Topic to produce to.
  pub topic: String,
  /// Client ID for the producer.
  pub client_id: Option<String>,
  /// Acks configuration ("0", "1", or "all").
  pub acks: String,
  /// Maximum number of retries.
  pub retries: u32,
  /// Batch size in bytes.
  pub batch_size: usize,
  /// Linger time in milliseconds (wait time before sending batch).
  pub linger_ms: u64,
  /// Maximum request size in bytes.
  pub max_request_size: usize,
  /// Compression type ("none", "gzip", "snappy", "lz4", "zstd").
  pub compression_type: String,
  /// Additional custom configuration properties.
  pub custom_properties: HashMap<String, String>,
}

impl Default for KafkaProducerConfig {
  fn default() -> Self {
    Self {
      bootstrap_servers: "localhost:9092".to_string(),
      topic: String::new(),
      client_id: None,
      acks: "all".to_string(),
      retries: 3,
      batch_size: 16384, // 16KB
      linger_ms: 0,
      max_request_size: 1048576, // 1MB
      compression_type: "none".to_string(),
      custom_properties: HashMap::new(),
    }
  }
}

impl KafkaProducerConfig {
  /// Sets the bootstrap servers.
  #[must_use]
  pub fn with_bootstrap_servers(mut self, servers: impl Into<String>) -> Self {
    self.bootstrap_servers = servers.into();
    self
  }

  /// Sets the topic to produce to.
  #[must_use]
  pub fn with_topic(mut self, topic: impl Into<String>) -> Self {
    self.topic = topic.into();
    self
  }

  /// Sets the client ID.
  #[must_use]
  pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
    self.client_id = Some(client_id.into());
    self
  }

  /// Sets the acks configuration.
  #[must_use]
  pub fn with_acks(mut self, acks: impl Into<String>) -> Self {
    self.acks = acks.into();
    self
  }

  /// Sets the number of retries.
  #[must_use]
  pub fn with_retries(mut self, retries: u32) -> Self {
    self.retries = retries;
    self
  }

  /// Sets the batch size.
  #[must_use]
  pub fn with_batch_size(mut self, size: usize) -> Self {
    self.batch_size = size;
    self
  }

  /// Sets the linger time.
  #[must_use]
  pub fn with_linger_ms(mut self, ms: u64) -> Self {
    self.linger_ms = ms;
    self
  }

  /// Sets the compression type.
  #[must_use]
  pub fn with_compression_type(mut self, compression: impl Into<String>) -> Self {
    self.compression_type = compression.into();
    self
  }

  /// Sets a custom property.
  #[must_use]
  pub fn with_custom_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.custom_properties.insert(key.into(), value.into());
    self
  }
}

/// A consumer that produces messages to Kafka topics.
///
/// This consumer serializes input elements and sends them to a Kafka topic.
/// It supports batching, retries, and various Kafka producer configurations.
///
/// # Example
///
/// ```ignore
/// use streamweave::consumers::kafka::kafka_consumer::KafkaConsumer;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let consumer = KafkaConsumer::<Event>::new(
///     KafkaProducerConfig::default()
///         .with_bootstrap_servers("localhost:9092")
///         .with_topic("my-topic")
/// );
/// ```
pub struct KafkaConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Consumer configuration.
  pub config: ConsumerConfig<T>,
  /// Kafka producer-specific configuration.
  pub kafka_config: KafkaProducerConfig,
}

impl<T> KafkaConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new Kafka consumer with the given configuration.
  #[must_use]
  pub fn new(kafka_config: KafkaProducerConfig) -> Self {
    Self {
      config: ConsumerConfig::default(),
      kafka_config,
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

  /// Returns the Kafka producer configuration.
  #[must_use]
  pub fn kafka_config(&self) -> &KafkaProducerConfig {
    &self.kafka_config
  }
}

impl<T> Clone for KafkaConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      kafka_config: self.kafka_config.clone(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use proptest::proptest;
  use serde::Serialize;

  #[derive(Debug, Clone, Serialize)]
  struct TestEvent {
    id: u32,
    message: String,
  }

  proptest! {
    #[test]
    fn test_kafka_producer_config_default(_ in prop::num::u8::ANY) {
      let config = KafkaProducerConfig::default();
      prop_assert_eq!(config.bootstrap_servers, "localhost:9092");
      prop_assert_eq!(config.acks, "all");
      prop_assert_eq!(config.retries, 3);
    }

    #[test]
    fn test_kafka_producer_config_builder(
      bootstrap_servers in "[a-zA-Z0-9.-]+:[0-9]+",
      topic in "[a-zA-Z0-9-_]+",
      client_id in prop::string::string_regex("[a-zA-Z0-9-_]+").unwrap(),
      acks in prop::sample::select(vec!["0", "1", "all"]),
      retries in 0u32..10u32,
      compression in prop::sample::select(vec!["none", "gzip", "snappy", "lz4", "zstd"])
    ) {
      let config = KafkaProducerConfig::default()
        .with_bootstrap_servers(bootstrap_servers.clone())
        .with_topic(topic.clone())
        .with_client_id(client_id.clone())
        .with_acks(acks)
        .with_retries(retries)
        .with_compression_type(compression);

      prop_assert_eq!(config.bootstrap_servers, bootstrap_servers);
      prop_assert_eq!(config.topic, topic);
      prop_assert_eq!(config.client_id, Some(client_id));
      prop_assert_eq!(config.acks.as_str(), acks);
      prop_assert_eq!(config.retries, retries);
      prop_assert_eq!(config.compression_type.as_str(), compression);
    }

    #[test]
    fn test_kafka_consumer_new(
      topic in "[a-zA-Z0-9-_]+"
    ) {
      let kafka_config = KafkaProducerConfig::default().with_topic(topic.clone());
      let consumer = KafkaConsumer::<TestEvent>::new(kafka_config);
      prop_assert_eq!(consumer.kafka_config().topic.as_str(), topic.as_str());
    }
  }
}

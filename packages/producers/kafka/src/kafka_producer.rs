use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use streamweave::ProducerConfig;
use streamweave_error::ErrorStrategy;

/// Configuration for Kafka consumer behavior.
#[derive(Debug, Clone)]
pub struct KafkaConsumerConfig {
  /// Bootstrap servers (comma-separated list of broker addresses).
  pub bootstrap_servers: String,
  /// Consumer group ID.
  pub group_id: Option<String>,
  /// Topics to consume from.
  pub topics: Vec<String>,
  /// Auto offset reset strategy ("earliest" or "latest").
  pub auto_offset_reset: String,
  /// Enable auto commit of offsets.
  pub enable_auto_commit: bool,
  /// Auto commit interval in milliseconds.
  pub auto_commit_interval_ms: u64,
  /// Session timeout in milliseconds.
  pub session_timeout_ms: u64,
  /// Maximum poll interval in milliseconds.
  pub max_poll_interval_ms: u64,
  /// Maximum number of bytes to fetch per request.
  pub fetch_max_bytes: usize,
  /// Maximum wait time for fetch requests in milliseconds.
  pub fetch_wait_max_ms: u64,
  /// Additional custom configuration properties.
  pub custom_properties: HashMap<String, String>,
}

impl Default for KafkaConsumerConfig {
  fn default() -> Self {
    Self {
      bootstrap_servers: "localhost:9092".to_string(),
      group_id: None,
      topics: Vec::new(),
      auto_offset_reset: "earliest".to_string(),
      enable_auto_commit: true,
      auto_commit_interval_ms: 5000,
      session_timeout_ms: 30000,
      max_poll_interval_ms: 300000,
      fetch_max_bytes: 1048576, // 1MB
      fetch_wait_max_ms: 500,
      custom_properties: HashMap::new(),
    }
  }
}

impl KafkaConsumerConfig {
  /// Sets the bootstrap servers.
  #[must_use]
  pub fn with_bootstrap_servers(mut self, servers: impl Into<String>) -> Self {
    self.bootstrap_servers = servers.into();
    self
  }

  /// Sets the consumer group ID.
  #[must_use]
  pub fn with_group_id(mut self, group_id: impl Into<String>) -> Self {
    self.group_id = Some(group_id.into());
    self
  }

  /// Adds a topic to consume from.
  #[must_use]
  pub fn with_topic(mut self, topic: impl Into<String>) -> Self {
    self.topics.push(topic.into());
    self
  }

  /// Sets multiple topics to consume from.
  #[must_use]
  pub fn with_topics(mut self, topics: Vec<String>) -> Self {
    self.topics = topics;
    self
  }

  /// Sets the auto offset reset strategy.
  #[must_use]
  pub fn with_auto_offset_reset(mut self, reset: impl Into<String>) -> Self {
    self.auto_offset_reset = reset.into();
    self
  }

  /// Sets whether to enable auto commit.
  #[must_use]
  pub fn with_enable_auto_commit(mut self, enable: bool) -> Self {
    self.enable_auto_commit = enable;
    self
  }

  /// Sets a custom property.
  #[must_use]
  pub fn with_custom_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.custom_properties.insert(key.into(), value.into());
    self
  }

  /// Sets the auto commit interval in milliseconds.
  #[must_use]
  pub fn with_auto_commit_interval_ms(mut self, interval_ms: u64) -> Self {
    self.auto_commit_interval_ms = interval_ms;
    self
  }
}

/// A message received from Kafka.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaMessage {
  /// Topic name.
  pub topic: String,
  /// Partition number.
  pub partition: i32,
  /// Offset.
  pub offset: i64,
  /// Message key (if any).
  pub key: Option<Vec<u8>>,
  /// Message payload.
  pub payload: Vec<u8>,
  /// Message timestamp.
  pub timestamp: Option<i64>,
  /// Message headers.
  pub headers: HashMap<String, Vec<u8>>,
}

/// A producer that consumes messages from Kafka topics.
///
/// This producer reads messages from one or more Kafka topics and yields
/// them as a stream. It supports consumer groups, partition assignment,
/// and offset management.
///
/// # Example
///
/// ```ignore
/// use streamweave::producers::kafka::kafka_producer::KafkaProducer;
///
/// let producer = KafkaProducer::new(
///     KafkaConsumerConfig::default()
///         .with_bootstrap_servers("localhost:9092")
///         .with_group_id("my-consumer-group")
///         .with_topic("my-topic")
/// );
/// ```
pub struct KafkaProducer {
  /// Kafka consumer configuration.
  pub config: ProducerConfig<KafkaMessage>,
  /// Kafka consumer-specific configuration.
  pub kafka_config: KafkaConsumerConfig,
}

impl KafkaProducer {
  /// Creates a new Kafka producer with the given configuration.
  #[must_use]
  pub fn new(kafka_config: KafkaConsumerConfig) -> Self {
    Self {
      config: ProducerConfig::default(),
      kafka_config,
    }
  }

  /// Sets the error strategy for the producer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<KafkaMessage>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the producer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the Kafka consumer configuration.
  #[must_use]
  pub fn kafka_config(&self) -> &KafkaConsumerConfig {
    &self.kafka_config
  }
}

impl Clone for KafkaProducer {
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

  #[test]
  fn test_kafka_consumer_config_default() {
    let config = KafkaConsumerConfig::default();
    assert_eq!(config.bootstrap_servers, "localhost:9092");
    assert!(config.topics.is_empty());
    assert_eq!(config.auto_offset_reset, "earliest");
    assert!(config.enable_auto_commit);
  }

  #[test]
  fn test_kafka_consumer_config_builder() {
    let config = KafkaConsumerConfig::default()
      .with_bootstrap_servers("kafka:9092")
      .with_group_id("test-group")
      .with_topic("test-topic")
      .with_auto_offset_reset("latest")
      .with_enable_auto_commit(false);

    assert_eq!(config.bootstrap_servers, "kafka:9092");
    assert_eq!(config.group_id, Some("test-group".to_string()));
    assert_eq!(config.topics.len(), 1);
    assert_eq!(config.topics[0], "test-topic");
    assert_eq!(config.auto_offset_reset, "latest");
    assert!(!config.enable_auto_commit);
  }

  #[test]
  fn test_kafka_producer_new() {
    let kafka_config = KafkaConsumerConfig::default().with_topic("test-topic");
    let producer = KafkaProducer::new(kafka_config);
    assert_eq!(producer.kafka_config().topics.len(), 1);
  }
}

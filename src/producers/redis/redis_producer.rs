use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for Redis Streams consumer behavior.
#[derive(Debug, Clone)]
pub struct RedisConsumerConfig {
  /// Redis connection URL (e.g., "redis://localhost:6379").
  pub connection_url: String,
  /// Stream name to consume from.
  pub stream: String,
  /// Consumer group name (optional, enables consumer groups).
  pub group: Option<String>,
  /// Consumer name (required if using consumer groups).
  pub consumer: Option<String>,
  /// Starting ID for reading (use "0" for beginning, "$" for new messages).
  pub start_id: String,
  /// Block time in milliseconds (0 for non-blocking).
  pub block_ms: u64,
  /// Count of messages to read per call.
  pub count: Option<usize>,
  /// Whether to acknowledge messages automatically.
  pub auto_ack: bool,
}

impl Default for RedisConsumerConfig {
  fn default() -> Self {
    Self {
      connection_url: "redis://localhost:6379".to_string(),
      stream: String::new(),
      group: None,
      consumer: None,
      start_id: "0".to_string(),
      block_ms: 1000,
      count: None,
      auto_ack: false,
    }
  }
}

impl RedisConsumerConfig {
  /// Sets the Redis connection URL.
  #[must_use]
  pub fn with_connection_url(mut self, url: impl Into<String>) -> Self {
    self.connection_url = url.into();
    self
  }

  /// Sets the stream name.
  #[must_use]
  pub fn with_stream(mut self, stream: impl Into<String>) -> Self {
    self.stream = stream.into();
    self
  }

  /// Sets the consumer group.
  #[must_use]
  pub fn with_group(mut self, group: impl Into<String>) -> Self {
    self.group = Some(group.into());
    self
  }

  /// Sets the consumer name.
  #[must_use]
  pub fn with_consumer(mut self, consumer: impl Into<String>) -> Self {
    self.consumer = Some(consumer.into());
    self
  }

  /// Sets the starting ID.
  #[must_use]
  pub fn with_start_id(mut self, id: impl Into<String>) -> Self {
    self.start_id = id.into();
    self
  }

  /// Sets the block time in milliseconds.
  #[must_use]
  pub fn with_block_ms(mut self, ms: u64) -> Self {
    self.block_ms = ms;
    self
  }

  /// Sets the count of messages per read.
  #[must_use]
  pub fn with_count(mut self, count: usize) -> Self {
    self.count = Some(count);
    self
  }

  /// Sets whether to auto-acknowledge messages.
  #[must_use]
  pub fn with_auto_ack(mut self, auto_ack: bool) -> Self {
    self.auto_ack = auto_ack;
    self
  }
}

/// A message received from Redis Streams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisMessage {
  /// Stream name.
  pub stream: String,
  /// Message ID.
  pub id: String,
  /// Message fields as key-value pairs.
  pub fields: HashMap<String, String>,
}

/// A producer that consumes messages from Redis Streams.
///
/// This producer reads messages from a Redis stream and yields them as
/// a stream. It supports consumer groups, message acknowledgment, and
/// pending message tracking.
///
/// # Example
///
/// ```ignore
/// use streamweave::producers::redis::{RedisProducer, RedisConsumerConfig};
///
/// let producer = RedisProducer::new(
///     RedisConsumerConfig::default()
///         .with_connection_url("redis://localhost:6379")
///         .with_stream("mystream")
///         .with_group("my-group")
///         .with_consumer("consumer-1")
/// );
/// ```
pub struct RedisProducer {
  /// Producer configuration.
  pub config: ProducerConfig<RedisMessage>,
  /// Redis Streams consumer-specific configuration.
  pub redis_config: RedisConsumerConfig,
}

impl RedisProducer {
  /// Creates a new Redis Streams producer with the given configuration.
  #[must_use]
  pub fn new(redis_config: RedisConsumerConfig) -> Self {
    Self {
      config: ProducerConfig::default(),
      redis_config,
    }
  }

  /// Sets the error strategy for the producer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<RedisMessage>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the producer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the Redis Streams consumer configuration.
  #[must_use]
  pub fn redis_config(&self) -> &RedisConsumerConfig {
    &self.redis_config
  }
}

impl Clone for RedisProducer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      redis_config: self.redis_config.clone(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_redis_consumer_config_default() {
    let config = RedisConsumerConfig::default();
    assert_eq!(config.connection_url, "redis://localhost:6379");
    assert_eq!(config.start_id, "0");
    assert_eq!(config.block_ms, 1000);
    assert!(!config.auto_ack);
  }

  #[test]
  fn test_redis_consumer_config_builder() {
    let config = RedisConsumerConfig::default()
      .with_connection_url("redis://redis:6379")
      .with_stream("test-stream")
      .with_group("test-group")
      .with_consumer("consumer-1")
      .with_start_id("$")
      .with_block_ms(5000)
      .with_count(100)
      .with_auto_ack(true);

    assert_eq!(config.connection_url, "redis://redis:6379");
    assert_eq!(config.stream, "test-stream");
    assert_eq!(config.group, Some("test-group".to_string()));
    assert_eq!(config.consumer, Some("consumer-1".to_string()));
    assert_eq!(config.start_id, "$");
    assert_eq!(config.block_ms, 5000);
    assert_eq!(config.count, Some(100));
    assert!(config.auto_ack);
  }

  #[test]
  fn test_redis_producer_new() {
    let redis_config = RedisConsumerConfig::default().with_stream("test-stream");
    let producer = RedisProducer::new(redis_config);
    assert_eq!(producer.redis_config().stream, "test-stream");
  }
}

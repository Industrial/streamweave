use crate::consumer::ConsumerConfig;
use crate::error::ErrorStrategy;
use serde::Serialize;
use std::collections::HashMap;

/// Configuration for Redis Streams producer behavior.
#[derive(Debug, Clone)]
pub struct RedisStreamsProducerConfig {
  /// Redis connection URL (e.g., "redis://localhost:6379").
  pub connection_url: String,
  /// Stream name to produce to.
  pub stream: String,
  /// Maximum length of the stream (trims old entries, None for no limit).
  pub maxlen: Option<usize>,
  /// Approximate maximum length (more efficient for large streams).
  pub approximate_maxlen: bool,
}

impl Default for RedisStreamsProducerConfig {
  fn default() -> Self {
    Self {
      connection_url: "redis://localhost:6379".to_string(),
      stream: String::new(),
      maxlen: None,
      approximate_maxlen: false,
    }
  }
}

impl RedisStreamsProducerConfig {
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

  /// Sets the maximum length of the stream.
  #[must_use]
  pub fn with_maxlen(mut self, maxlen: usize) -> Self {
    self.maxlen = Some(maxlen);
    self
  }

  /// Sets whether to use approximate maxlen (more efficient).
  #[must_use]
  pub fn with_approximate_maxlen(mut self, approximate: bool) -> Self {
    self.approximate_maxlen = approximate;
    self
  }
}

/// A consumer that produces messages to Redis Streams.
///
/// This consumer serializes input elements and sends them to a Redis stream
/// using XADD. It supports configurable stream length limits and automatic
/// field mapping.
///
/// # Example
///
/// ```ignore
/// use streamweave::consumers::redis_streams::{RedisStreamsConsumer, RedisStreamsProducerConfig};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let consumer = RedisStreamsConsumer::<Event>::new(
///     RedisStreamsProducerConfig::default()
///         .with_connection_url("redis://localhost:6379")
///         .with_stream("mystream")
/// );
/// ```
pub struct RedisStreamsConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Consumer configuration.
  pub config: ConsumerConfig<T>,
  /// Redis Streams producer-specific configuration.
  pub redis_config: RedisStreamsProducerConfig,
}

impl<T> RedisStreamsConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new Redis Streams consumer with the given configuration.
  #[must_use]
  pub fn new(redis_config: RedisStreamsProducerConfig) -> Self {
    Self {
      config: ConsumerConfig::default(),
      redis_config,
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

  /// Returns the Redis Streams producer configuration.
  #[must_use]
  pub fn redis_config(&self) -> &RedisStreamsProducerConfig {
    &self.redis_config
  }
}

impl<T> Clone for RedisStreamsConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
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
  use serde::Serialize;

  #[derive(Debug, Clone, Serialize)]
  struct TestEvent {
    id: u32,
    message: String,
  }

  #[test]
  fn test_redis_streams_producer_config_default() {
    let config = RedisStreamsProducerConfig::default();
    assert_eq!(config.connection_url, "redis://localhost:6379");
    assert_eq!(config.maxlen, None);
    assert!(!config.approximate_maxlen);
  }

  #[test]
  fn test_redis_streams_producer_config_builder() {
    let config = RedisStreamsProducerConfig::default()
      .with_connection_url("redis://redis:6379")
      .with_stream("test-stream")
      .with_maxlen(1000)
      .with_approximate_maxlen(true);

    assert_eq!(config.connection_url, "redis://redis:6379");
    assert_eq!(config.stream, "test-stream");
    assert_eq!(config.maxlen, Some(1000));
    assert!(config.approximate_maxlen);
  }

  #[test]
  fn test_redis_streams_consumer_new() {
    let redis_config = RedisStreamsProducerConfig::default().with_stream("test-stream");
    let consumer = RedisStreamsConsumer::<TestEvent>::new(redis_config);
    assert_eq!(consumer.redis_config().stream, "test-stream");
  }
}

use serde::Serialize;
use streamweave_core::ConsumerConfig;
use streamweave_error::ErrorStrategy;

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
    fn test_redis_streams_producer_config_default(_ in prop::num::u8::ANY) {
      let config = RedisStreamsProducerConfig::default();
      prop_assert_eq!(config.connection_url, "redis://localhost:6379");
      prop_assert_eq!(config.maxlen, None);
      prop_assert!(!config.approximate_maxlen);
    }

    #[test]
    fn test_redis_streams_producer_config_builder(
      connection_url in "redis://[a-zA-Z0-9.-]+:[0-9]+",
      stream in "[a-zA-Z0-9-_]+",
      maxlen in prop::option::of(1000usize..100000usize),
      approximate_maxlen in prop::bool::ANY
    ) {
      let config = RedisStreamsProducerConfig::default()
        .with_connection_url(connection_url.clone())
        .with_stream(stream.clone())
        .with_approximate_maxlen(approximate_maxlen);

      let config = if let Some(max) = maxlen {
        config.with_maxlen(max)
      } else {
        config
      };

      prop_assert_eq!(config.connection_url, connection_url);
      prop_assert_eq!(config.stream, stream);
      prop_assert_eq!(config.maxlen, maxlen);
      prop_assert_eq!(config.approximate_maxlen, approximate_maxlen);
    }

    #[test]
    fn test_redis_streams_consumer_new(
      stream in "[a-zA-Z0-9-_]+"
    ) {
      let redis_config = RedisStreamsProducerConfig::default().with_stream(stream.clone());
      let consumer = RedisStreamsConsumer::<TestEvent>::new(redis_config);
      prop_assert_eq!(consumer.redis_config().stream.as_str(), stream.as_str());
    }
  }
}

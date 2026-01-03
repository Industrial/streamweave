use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use streamweave::error::ErrorStrategy;
use streamweave::{Output, Producer, ProducerConfig};

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
/// use streamweave_redis::{RedisProducer, RedisConsumerConfig};
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

// Output trait implementation
use futures::Stream;
use std::pin::Pin;

impl Output for RedisProducer {
  type Output = RedisMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

// Producer trait implementation
#[cfg(feature = "redis")]
use async_stream::stream;
#[cfg(feature = "redis")]
use async_trait::async_trait;
#[cfg(feature = "redis")]
use redis::{
  AsyncCommands, Client, RedisResult,
  aio::ConnectionManager,
  streams::{StreamReadOptions, StreamReadReply},
};
#[cfg(feature = "redis")]
use std::time::Duration;
#[cfg(feature = "redis")]
use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};
#[cfg(feature = "redis")]
use tokio::time::sleep;
#[cfg(feature = "redis")]
use tracing::{error, warn};

#[async_trait]
#[cfg(feature = "redis")]
#[allow(clippy::collapsible_if)]
impl Producer for RedisProducer {
  type OutputPorts = (RedisMessage,);

  /// Produces a stream of messages from Redis Streams.
  ///
  /// # Error Handling
  ///
  /// - Connection errors are handled according to the error strategy.
  /// - Read errors trigger retries based on the error strategy.
  fn produce(&mut self) -> Self::OutputStream {
    let redis_config = self.redis_config.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "redis_producer".to_string());
    let error_strategy = self.config.error_strategy.clone();

    Box::pin(stream! {
      // Connect to Redis
      let client: Client = match Client::open(redis_config.connection_url.as_str()) {
        Ok(c) => c,
        Err(e) => {
          error!(
            component = %component_name,
            error = %e,
            "Failed to create Redis client, producing empty stream"
          );
          return;
        }
      };

      let mut connection: ConnectionManager = match client.get_connection_manager().await {
        Ok(conn) => conn,
        Err(e) => {
          error!(
            component = %component_name,
            error = %e,
            "Failed to connect to Redis, producing empty stream"
          );
          return;
        }
      };

      let stream_name = redis_config.stream.clone();
      let mut start_id = redis_config.start_id.clone();

      // Create consumer group if specified
      if let (Some(group), Some(_consumer)) = (&redis_config.group, &redis_config.consumer) {
        // Try to create consumer group (ignore if it already exists)
        let _: RedisResult<()> = connection.xgroup_create(&stream_name, group, &start_id).await;
        start_id = ">".to_string(); // Use '>' to read new messages in consumer group
      }

      // Poll for messages
      loop {
        let _read_options = StreamReadOptions::default()
          .count(redis_config.count.unwrap_or(1))
          .block(redis_config.block_ms.try_into().unwrap());

        let result: RedisResult<StreamReadReply> = if let (Some(group), Some(consumer)) = (&redis_config.group, &redis_config.consumer) {
          // Read from consumer group using command interface
          redis::cmd("XREADGROUP")
            .arg("GROUP")
            .arg(group)
            .arg(consumer)
            .arg("COUNT")
            .arg(redis_config.count.unwrap_or(1))
            .arg("BLOCK")
            .arg(redis_config.block_ms)
            .arg("STREAMS")
            .arg(&stream_name)
            .arg(&start_id)
            .query_async(&mut connection)
            .await
        } else {
          // Simple XREAD
          redis::cmd("XREAD")
            .arg("COUNT")
            .arg(redis_config.count.unwrap_or(1))
            .arg("BLOCK")
            .arg(redis_config.block_ms)
            .arg("STREAMS")
            .arg(&stream_name)
            .arg(&start_id)
            .query_async(&mut connection)
            .await
        };

        match result {
          Ok(reply) => {
            for stream_key in reply.keys {
              for stream_id in stream_key.ids {
                let mut fields = HashMap::new();
                for (field, value) in stream_id.map.iter() {
                  fields.insert(
                    field.clone(),
                    format!("{:?}", value),
                  );
                }

                let message = RedisMessage {
                  stream: stream_key.key.clone(),
                  id: stream_id.id.clone(),
                  fields,
                };

                // Acknowledge message if auto-ack is enabled and using consumer groups
                if redis_config.auto_ack {
                  if let Some(ref group) = redis_config.group {
                    if let Err(e) = connection.xack::<&str, &str, &str, ()>(&stream_name, group, &[&stream_id.id]).await {
                      warn!(
                        component = %component_name,
                        message_id = %stream_id.id,
                        error = %e,
                        "Failed to acknowledge message"
                      );
                    }
                  }
                }

                yield message;
                start_id = stream_id.id.clone();
              }
            }
          }
          Err(e) => {
            let error = StreamError::new(
              Box::new(e),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: None,
                component_name: component_name.clone(),
                component_type: std::any::type_name::<Self>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<Self>().to_string(),
              },
            );

            match handle_error_strategy(&error_strategy, &error) {
              ErrorAction::Stop => {
                error!(
                  component = %component_name,
                  error = %error,
                  "Stopping due to Redis read error"
                );
                break;
              }
              ErrorAction::Skip => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Skipping due to Redis read error, continuing to poll"
                );
                sleep(Duration::from_millis(100)).await;
              }
              ErrorAction::Retry => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Retrying Redis read after delay"
                );
                sleep(Duration::from_millis(1000)).await;
              }
            }
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<RedisMessage>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<RedisMessage> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<RedisMessage> {
    &mut self.config
  }
}

#[cfg(feature = "redis")]
fn handle_error_strategy<T>(
  strategy: &streamweave::error::ErrorStrategy<T>,
  error: &StreamError<T>,
) -> ErrorAction
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
  match strategy {
    streamweave::error::ErrorStrategy::Stop => ErrorAction::Stop,
    streamweave::error::ErrorStrategy::Skip => ErrorAction::Skip,
    streamweave::error::ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    streamweave::error::ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}

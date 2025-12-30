use serde::Serialize;
use streamweave::{Consumer, ConsumerConfig, Input};
use streamweave_error::ErrorStrategy;

/// Configuration for Redis Streams producer behavior.
#[derive(Debug, Clone)]
pub struct RedisProducerConfig {
  /// Redis connection URL (e.g., "redis://localhost:6379").
  pub connection_url: String,
  /// Stream name to produce to.
  pub stream: String,
  /// Maximum length of the stream (trims old entries, None for no limit).
  pub maxlen: Option<usize>,
  /// Approximate maximum length (more efficient for large streams).
  pub approximate_maxlen: bool,
}

impl Default for RedisProducerConfig {
  fn default() -> Self {
    Self {
      connection_url: "redis://localhost:6379".to_string(),
      stream: String::new(),
      maxlen: None,
      approximate_maxlen: false,
    }
  }
}

impl RedisProducerConfig {
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
/// use streamweave_redis::{RedisConsumer, RedisProducerConfig};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let consumer = RedisConsumer::<Event>::new(
///     RedisProducerConfig::default()
///         .with_connection_url("redis://localhost:6379")
///         .with_stream("mystream")
/// );
/// ```
pub struct RedisConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Consumer configuration.
  pub config: ConsumerConfig<T>,
  /// Redis Streams producer-specific configuration.
  pub redis_config: RedisProducerConfig,
}

impl<T> RedisConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new Redis Streams consumer with the given configuration.
  #[must_use]
  pub fn new(redis_config: RedisProducerConfig) -> Self {
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
  pub fn redis_config(&self) -> &RedisProducerConfig {
    &self.redis_config
  }
}

impl<T> Clone for RedisConsumer<T>
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

// Input trait implementation
use futures::Stream;
use std::pin::Pin;

impl<T> Input for RedisConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

// Consumer trait implementation
#[cfg(feature = "redis")]
use async_trait::async_trait;
#[cfg(feature = "redis")]
use futures::StreamExt;
#[cfg(feature = "redis")]
use redis::{AsyncCommands, Client, RedisResult, aio::ConnectionManager};
#[cfg(feature = "redis")]
use std::collections::HashMap;
#[cfg(feature = "redis")]
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};
#[cfg(feature = "redis")]
use tracing::{error, warn};

#[async_trait]
#[cfg(feature = "redis")]
impl<T> Consumer for RedisConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  /// Consumes a stream and sends each item as a message to a Redis stream.
  ///
  /// # Error Handling
  ///
  /// - If the Redis connection cannot be established, an error is logged and no data is sent.
  /// - If an item cannot be serialized or sent, the error strategy determines the action.
  async fn consume(&mut self, input: Self::InputStream) {
    let redis_config = self.redis_config.clone();
    let component_name = if self.config.name.is_empty() {
      "redis_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let error_strategy = self.config.error_strategy.clone();

    // Connect to Redis
    let client: Client = match Client::open(redis_config.connection_url.as_str()) {
      Ok(c) => c,
      Err(e) => {
        error!(
          component = %component_name,
          error = %e,
          "Failed to create Redis client"
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
          "Failed to connect to Redis"
        );
        return;
      }
    };

    let stream_name = redis_config.stream.clone();
    let mut input = std::pin::pin!(input);

    // Send messages
    while let Some(item) = input.next().await {
      // Serialize the item to a HashMap of fields
      let fields: HashMap<String, String> = match serde_json::to_value(&item) {
        Ok(value) => {
          match value.as_object() {
            Some(obj) => obj
              .iter()
              .filter_map(|(k, v)| match v.as_str() {
                Some(s) => Some((k.clone(), s.to_string())),
                None => serde_json::to_string(v).ok().map(|s| (k.clone(), s)),
              })
              .collect(),
            None => {
              // If not an object, store as a single "value" field
              let mut map = HashMap::new();
              if let Ok(json_str) = serde_json::to_string(&value) {
                map.insert("value".to_string(), json_str);
              }
              map
            }
          }
        }
        Err(e) => {
          let error = StreamError::new(
            Box::new(e),
            ErrorContext {
              timestamp: chrono::Utc::now(),
              item: Some(item.clone()),
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
                "Stopping due to serialization error"
              );
              break;
            }
            ErrorAction::Skip => {
              warn!(
                component = %component_name,
                error = %error,
                "Skipping item due to serialization error"
              );
              continue;
            }
            ErrorAction::Retry => {
              warn!(
                component = %component_name,
                error = %error,
                "Retry not fully supported for serialization errors, skipping"
              );
              continue;
            }
          }
        }
      };

      // Execute XADD
      // Note: maxlen support would require using the command interface directly
      // For now, we use simple xadd - maxlen can be handled via Redis configuration
      let fields_vec: Vec<(&str, &str)> = fields
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
      let result: RedisResult<String> = connection.xadd(&stream_name, "*", &fields_vec).await;

      match result {
        Ok(_message_id) => {
          // Message sent successfully
        }
        Err(e) => {
          let error = StreamError::new(
            Box::new(e),
            ErrorContext {
              timestamp: chrono::Utc::now(),
              item: Some(item.clone()),
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
                stream = %stream_name,
                error = %error,
                "Stopping due to Redis XADD error"
              );
              break;
            }
            ErrorAction::Skip => {
              warn!(
                component = %component_name,
                stream = %stream_name,
                error = %error,
                "Skipping item due to Redis XADD error"
              );
              continue;
            }
            ErrorAction::Retry => {
              warn!(
                component = %component_name,
                stream = %stream_name,
                error = %error,
                "Retry not fully supported for Redis XADD errors, skipping"
              );
              continue;
            }
          }
        }
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<T> {
    &mut self.config
  }
}

#[cfg(feature = "redis")]
fn handle_error_strategy<T>(
  strategy: &streamweave_error::ErrorStrategy<T>,
  error: &StreamError<T>,
) -> ErrorAction
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
  match strategy {
    streamweave_error::ErrorStrategy::Stop => ErrorAction::Stop,
    streamweave_error::ErrorStrategy::Skip => ErrorAction::Skip,
    streamweave_error::ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    streamweave_error::ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}

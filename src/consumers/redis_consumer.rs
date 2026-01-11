//! Redis consumer for writing stream data to Redis Streams.
//!
//! This module provides [`RedisConsumer`], a consumer that writes stream items
//! to Redis Streams using the XADD command. Redis Streams are a log-like data
//! structure that supports message queuing, event sourcing, and stream processing
//! patterns.
//!
//! # Overview
//!
//! [`RedisConsumer`] is useful for publishing stream data to Redis Streams for
//! distributed message processing, event sourcing, and integration with Redis-based
//! systems. It serializes items and writes them to Redis streams with configurable
//! stream length limits.
//!
//! # Key Concepts
//!
//! - **Redis Streams**: Uses Redis Streams data structure (XADD command)
//! - **Serializable Items**: Items must implement `Serialize` for Redis serialization
//! - **Stream Length Limits**: Supports configurable maximum stream lengths
//! - **Event Sourcing**: Suitable for event sourcing and message queuing patterns
//!
//! # Core Types
//!
//! - **[`RedisConsumer<T>`]**: Consumer that writes items to Redis Streams
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::RedisConsumer;
//! use streamweave::producers::RedisProducerConfig;
//! use futures::stream;
//! use serde::Serialize;
//!
//! #[derive(Serialize, Clone, Debug)]
//! struct Event {
//!     id: u32,
//!     message: String,
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create Redis configuration
//! let redis_config = RedisProducerConfig {
//!     connection_string: "redis://localhost:6379".to_string(),
//!     stream_key: "events".to_string(),
//!     maxlen: Some(10000),
//!     // ... other configuration
//! };
//!
//! // Create a consumer
//! let mut consumer = RedisConsumer::<Event>::new(redis_config);
//!
//! // Create a stream of events
//! let stream = stream::iter(vec![
//!     Event { id: 1, message: "event1".to_string() },
//!     Event { id: 2, message: "event2".to_string() },
//! ]);
//!
//! // Consume the stream (events written to Redis Streams)
//! consumer.consume(Box::pin(stream)).await;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::consumers::RedisConsumer;
//! use streamweave::producers::RedisProducerConfig;
//! use streamweave::ErrorStrategy;
//!
//! # use serde::Serialize;
//! # #[derive(Serialize, Clone, Debug)]
//! # struct Event { id: u32, message: String }
//! let redis_config = RedisProducerConfig { /* ... */ };
//! let consumer = RedisConsumer::<Event>::new(redis_config)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("redis-writer".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Redis Streams**: Uses Redis Streams for efficient, persistent message storage
//! - **XADD Command**: Uses XADD for adding messages to streams
//! - **Stream Length Limits**: Supports MAXLEN for bounded streams
//! - **Serialize Requirement**: Items must implement `Serialize` for flexible data
//!   structure support
//!
//! # Integration with StreamWeave
//!
//! [`RedisConsumer`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ConsumerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::producers::RedisProducerConfig;
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use redis::{AsyncCommands, Client, RedisResult, aio::ConnectionManager};
use serde::Serialize;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, warn};

/// A consumer that writes data to Redis Streams.
///
/// This consumer serializes items and writes them to a Redis stream using XADD.
/// It supports stream length limits and approximate maxlen for efficiency.
pub struct RedisConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Consumer configuration.
  pub config: ConsumerConfig<T>,
  /// Redis producer-specific configuration.
  pub redis_config: RedisProducerConfig,
  /// Connection handle (shared across stream items).
  connection: Arc<Mutex<Option<ConnectionManager>>>,
}

impl<T> RedisConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new Redis consumer with the given configuration.
  #[must_use]
  pub fn new(redis_config: RedisProducerConfig) -> Self {
    Self {
      config: ConsumerConfig::default(),
      redis_config,
      connection: Arc::new(Mutex::new(None)),
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

  /// Returns the Redis producer configuration.
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
      connection: Arc::clone(&self.connection),
    }
  }
}

impl<T> Input for RedisConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

#[async_trait]
impl<T> Consumer for RedisConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) {
    let component_name = if self.config.name.is_empty() {
      "redis_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let error_strategy = self.config.error_strategy.clone();
    let redis_config = self.redis_config.clone();
    let connection = Arc::clone(&self.connection);
    let stream_name = redis_config.stream.clone();

    // Initialize connection on first item
    {
      let mut connection_guard = connection.lock().await;
      if connection_guard.is_none() {
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

        match client.get_connection_manager().await {
          Ok(conn) => {
            *connection_guard = Some(conn);
          }
          Err(e) => {
            error!(
              component = %component_name,
              error = %e,
              "Failed to connect to Redis"
            );
            return;
          }
        }
      }
    }

    while let Some(item) = stream.next().await {
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
          let stream_error = StreamError::new(
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

          let action = match &error_strategy {
            ErrorStrategy::Stop => ErrorAction::Stop,
            ErrorStrategy::Skip => ErrorAction::Skip,
            ErrorStrategy::Retry(n) if stream_error.retries < *n => ErrorAction::Retry,
            ErrorStrategy::Custom(handler) => handler(&stream_error),
            _ => ErrorAction::Stop,
          };

          match action {
            ErrorAction::Stop => {
              error!(
                component = %component_name,
                error = %stream_error,
                "Stopping due to serialization error"
              );
              return;
            }
            ErrorAction::Skip => {
              warn!(
                component = %component_name,
                error = %stream_error,
                "Skipping item due to serialization error"
              );
              continue;
            }
            ErrorAction::Retry => {
              warn!(
                component = %component_name,
                error = %stream_error,
                "Retry not supported for serialization errors, skipping"
              );
              continue;
            }
          }
        }
      };

      // Execute XADD with optional MAXLEN
      {
        let mut connection_guard = connection.lock().await;
        if let Some(ref mut conn) = *connection_guard {
          let fields_vec: Vec<(&str, &str)> = fields
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

          // Build XADD command with optional MAXLEN
          let result: RedisResult<String> = if let Some(maxlen) = redis_config.maxlen {
            // Use raw command for MAXLEN support
            let mut cmd = redis::cmd("XADD");
            cmd.arg(&stream_name);
            if redis_config.approximate_maxlen {
              // Use approximate maxlen: XADD stream MAXLEN ~ count * ...
              cmd.arg("MAXLEN").arg("~").arg(maxlen);
            } else {
              // Use exact maxlen: XADD stream MAXLEN count * ...
              cmd.arg("MAXLEN").arg(maxlen);
            }
            cmd.arg("*");
            for (k, v) in &fields_vec {
              cmd.arg(k).arg(v);
            }
            cmd.query_async(conn).await
          } else {
            // No maxlen, just add
            conn.xadd(&stream_name, "*", &fields_vec).await
          };

          match result {
            Ok(_message_id) => {
              // Message sent successfully
            }
            Err(e) => {
              let stream_error = StreamError::new(
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

              let action = match &error_strategy {
                ErrorStrategy::Stop => ErrorAction::Stop,
                ErrorStrategy::Skip => ErrorAction::Skip,
                ErrorStrategy::Retry(n) if stream_error.retries < *n => ErrorAction::Retry,
                ErrorStrategy::Custom(handler) => handler(&stream_error),
                _ => ErrorAction::Stop,
              };

              match action {
                ErrorAction::Stop => {
                  error!(
                    component = %component_name,
                    stream = %stream_name,
                    error = %stream_error,
                    "Stopping due to Redis XADD error"
                  );
                  return;
                }
                ErrorAction::Skip => {
                  warn!(
                    component = %component_name,
                    stream = %stream_name,
                    error = %stream_error,
                    "Skipping item due to Redis XADD error"
                  );
                  continue;
                }
                ErrorAction::Retry => {
                  warn!(
                    component = %component_name,
                    stream = %stream_name,
                    error = %stream_error,
                    "Retry not supported for Redis XADD errors, skipping"
                  );
                  continue;
                }
              }
            }
          }
        }
      }
    }
  }

  fn get_config_impl(&self) -> &ConsumerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<T> {
    &mut self.config
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }
}

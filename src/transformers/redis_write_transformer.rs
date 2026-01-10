//! Redis write transformer for StreamWeave
//!
//! Writes data to Redis Streams while passing data through. Takes serializable data as input,
//! writes to Redis, and outputs the same data, enabling writing to Redis and continuing processing.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::producers::RedisProducerConfig;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use redis::{AsyncCommands, Client, RedisResult, aio::ConnectionManager};
use serde::Serialize;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, warn};

/// A transformer that writes data to Redis Streams while passing data through.
///
/// Each input item is serialized and written to a Redis stream using XADD,
/// and then the same item is output, enabling writing to Redis and continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::RedisWriteTransformer;
/// use streamweave::consumers::RedisProducerConfig;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let config = RedisProducerConfig::default()
///   .with_connection_url("redis://localhost:6379")
///   .with_stream("events");
/// let transformer = RedisWriteTransformer::<Event>::new(config);
/// // Input: [Event, ...]
/// // Writes to Redis and outputs: [Event, ...]
/// ```
pub struct RedisWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Redis Streams producer configuration.
  redis_config: RedisProducerConfig,
  /// Transformer configuration.
  config: TransformerConfig<T>,
  /// Connection handle (shared across stream items).
  connection: Arc<Mutex<Option<ConnectionManager>>>,
}

impl<T> RedisWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `RedisWriteTransformer` with the given Redis configuration.
  ///
  /// # Arguments
  ///
  /// * `redis_config` - Redis Streams producer configuration.
  #[must_use]
  pub fn new(redis_config: RedisProducerConfig) -> Self {
    Self {
      redis_config,
      config: TransformerConfig::default(),
      connection: Arc::new(Mutex::new(None)),
    }
  }

  /// Sets the error handling strategy for this transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the Redis Streams producer configuration.
  #[must_use]
  pub fn redis_config(&self) -> &RedisProducerConfig {
    &self.redis_config
  }
}

impl<T> Clone for RedisWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      redis_config: self.redis_config.clone(),
      config: self.config.clone(),
      connection: Arc::clone(&self.connection),
    }
  }
}

impl<T> Input for RedisWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for RedisWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for RedisWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "redis_write_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let redis_config = self.redis_config.clone();
    let connection = Arc::clone(&self.connection);
    let stream_name = redis_config.stream.clone();

    Box::pin(async_stream::stream! {
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

      let mut input_stream = input;
      while let Some(item) = input_stream.next().await {
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
                component_type: std::any::type_name::<RedisWriteTransformer<T>>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<RedisWriteTransformer<T>>().to_string(),
              },
            );

            match handle_error_strategy(&error_strategy, &stream_error) {
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

        // Execute XADD
        {
          let mut connection_guard = connection.lock().await;
          if let Some(ref mut conn) = *connection_guard {
            let fields_vec: Vec<(&str, &str)> = fields
              .iter()
              .map(|(k, v)| (k.as_str(), v.as_str()))
              .collect();
            let result: RedisResult<String> = conn.xadd(&stream_name, "*", &fields_vec).await;

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
                    component_type: std::any::type_name::<RedisWriteTransformer<T>>().to_string(),
                  },
                  ComponentInfo {
                    name: component_name.clone(),
                    type_name: std::any::type_name::<RedisWriteTransformer<T>>().to_string(),
                  },
                );

                match handle_error_strategy(&error_strategy, &stream_error) {
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

        // Pass through the item
        yield item;
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "redis_write_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "redis_write_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

/// Helper function to handle error strategy
pub(crate) fn handle_error_strategy<T>(
  strategy: &ErrorStrategy<T>,
  error: &StreamError<T>,
) -> ErrorAction
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
  match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    ErrorStrategy::Skip => ErrorAction::Skip,
    ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}

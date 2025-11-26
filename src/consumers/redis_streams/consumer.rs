#[cfg(all(not(target_arch = "wasm32"), feature = "redis-streams"))]
use super::redis_streams_consumer::{RedisStreamsConsumer, RedisStreamsProducerConfig};
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
#[cfg(all(not(target_arch = "wasm32"), feature = "redis-streams"))]
use async_trait::async_trait;
#[cfg(all(not(target_arch = "wasm32"), feature = "redis-streams"))]
use futures::StreamExt;
#[cfg(all(not(target_arch = "wasm32"), feature = "redis-streams"))]
use redis::{AsyncCommands, Client, RedisResult, aio::ConnectionManager};
#[cfg(all(not(target_arch = "wasm32"), feature = "redis-streams"))]
use serde::Serialize;
#[cfg(all(not(target_arch = "wasm32"), feature = "redis-streams"))]
use std::collections::HashMap;
#[cfg(all(not(target_arch = "wasm32"), feature = "redis-streams"))]
use tracing::{error, warn};

#[async_trait]
#[cfg(all(not(target_arch = "wasm32"), feature = "redis-streams"))]
impl<T> Consumer for RedisStreamsConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Consumes a stream and sends each item as a message to a Redis stream.
  ///
  /// # Error Handling
  ///
  /// - If the Redis connection cannot be established, an error is logged and no data is sent.
  /// - If an item cannot be serialized or sent, the error strategy determines the action.
  async fn consume(&mut self, input: Self::InputStream) {
    let redis_config = self.redis_config.clone();
    let component_name = if self.config.name.is_empty() {
      "redis_streams_consumer".to_string()
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

    let mut connection: ConnectionManager = match client.get_tokio_connection_manager().await {
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
      let result: RedisResult<String> = connection.xadd(&stream_name, "*", &fields).await;

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

#[cfg(all(not(target_arch = "wasm32"), feature = "redis-streams"))]
fn handle_error_strategy<T>(strategy: &ErrorStrategy<T>, error: &StreamError<T>) -> ErrorAction
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

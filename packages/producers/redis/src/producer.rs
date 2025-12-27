#[cfg(feature = "redis")]
use super::redis_producer::{RedisMessage, RedisProducer};
#[cfg(feature = "redis")]
use async_stream::stream;
#[cfg(feature = "redis")]
use async_trait::async_trait;
use streamweave::{Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

#[cfg(feature = "redis")]
use redis::{
  AsyncCommands, Client, RedisResult,
  aio::ConnectionManager,
  streams::{StreamReadOptions, StreamReadReply},
};
#[cfg(feature = "redis")]
use std::collections::HashMap;
#[cfg(feature = "redis")]
use std::time::Duration;
#[cfg(feature = "redis")]
use tokio::time::sleep;
#[cfg(feature = "redis")]
use tracing::{error, warn};

#[async_trait]
#[cfg(feature = "redis")]
#[allow(clippy::collapsible_if)]
impl Producer for RedisProducer {
  type OutputPorts = (crate::redis_producer::RedisMessage,);

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

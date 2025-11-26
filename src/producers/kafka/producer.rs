#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use super::kafka_producer::{KafkaConsumerConfig, KafkaMessage, KafkaProducer};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::producer::{Producer, ProducerConfig};
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use async_stream::stream;
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use async_trait::async_trait;
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use futures::StreamExt;
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use rdkafka::{
  ClientContext, Statistics,
  config::{ClientConfig, RDKafkaLogLevel},
  consumer::stream_consumer::StreamConsumer,
  consumer::{Consumer, ConsumerContext},
  message::{BorrowedMessage, Headers, Message},
};
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use serde::Deserialize;
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use std::sync::Arc;
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use std::time::Duration;
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use tokio::time::sleep;
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use tracing::{error, warn};

#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
struct StreamWeaveConsumerContext;

#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
impl ClientContext for StreamWeaveConsumerContext {
  fn stats(&self, _statistics: Statistics) {
    // Statistics can be logged or collected here if needed
  }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
impl ConsumerContext for StreamWeaveConsumerContext {}

#[async_trait]
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
impl Producer for KafkaProducer<KafkaMessage> {
  /// Produces a stream of messages from Kafka topics.
  ///
  /// # Error Handling
  ///
  /// - Connection errors are handled according to the error strategy.
  /// - Deserialization errors are handled according to the error strategy.
  /// - Network errors trigger retries based on the error strategy.
  fn produce(&mut self) -> Self::OutputStream {
    let kafka_config = self.kafka_config.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "kafka_producer".to_string());
    let error_strategy = self.config.error_strategy.clone();

    Box::pin(stream! {
      // Build Kafka client configuration
      let mut client_config = ClientConfig::new();
      client_config.set("bootstrap.servers", &kafka_config.bootstrap_servers);
      client_config.set("enable.partition.eof", "false");
      client_config.set_log_level(RDKafkaLogLevel::Warning);

      if let Some(ref group_id) = kafka_config.group_id {
        client_config.set("group.id", group_id);
      }

      // Set auto offset reset
      client_config.set("auto.offset.reset", &kafka_config.auto_offset_reset);

      // Set auto commit
      client_config.set(
        "enable.auto.commit",
        if kafka_config.enable_auto_commit {
          "true"
        } else {
          "false"
        },
      );
      client_config.set(
        "auto.commit.interval.ms",
        &kafka_config.auto_commit_interval_ms.to_string(),
      );

      // Set session and poll timeouts
      client_config.set("session.timeout.ms", &kafka_config.session_timeout_ms.to_string());
      client_config.set(
        "max.poll.interval.ms",
        &kafka_config.max_poll_interval_ms.to_string(),
      );

      // Set fetch settings
      client_config.set("fetch.max.bytes", &kafka_config.fetch_max_bytes.to_string());
      client_config.set("fetch.wait.max.ms", &kafka_config.fetch_wait_max_ms.to_string());

      // Apply custom properties
      for (key, value) in &kafka_config.custom_properties {
        client_config.set(key, value);
      }

      // Create consumer
      let consumer: StreamConsumer<StreamWeaveConsumerContext> = match client_config
        .create_with_context(StreamWeaveConsumerContext)
      {
        Ok(c) => c,
        Err(e) => {
          error!(
            component = %component_name,
            error = %e,
            "Failed to create Kafka consumer, producing empty stream"
          );
          return;
        }
      };

      // Subscribe to topics
      if let Err(e) = consumer.subscribe(&kafka_config.topics.iter().map(|s| s.as_str()).collect::<Vec<_>>()) {
        error!(
          component = %component_name,
          topics = ?kafka_config.topics,
          error = %e,
          "Failed to subscribe to Kafka topics, producing empty stream"
        );
        return;
      }

      // Poll for messages
      loop {
        match consumer.recv().await {
          Ok(message) => {
            let kafka_message = convert_message(&message);
            yield kafka_message;
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
                  "Stopping due to Kafka receive error"
                );
                break;
              }
              ErrorAction::Skip => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Skipping due to Kafka receive error, continuing to poll"
                );
                sleep(Duration::from_millis(100)).await;
              }
              ErrorAction::Retry => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Retrying Kafka receive after delay"
                );
                sleep(Duration::from_millis(1000)).await;
              }
            }
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<KafkaMessage>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<KafkaMessage> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<KafkaMessage> {
    &mut self.config
  }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
fn convert_message(message: &BorrowedMessage<'_>) -> KafkaMessage {
  let mut headers = std::collections::HashMap::new();
  if let Some(message_headers) = message.headers() {
    for header in message_headers.iter() {
      if let (Some(key), Some(value)) = (header.key, header.value) {
        headers.insert(key.to_string(), value.to_vec());
      }
    }
  }

  KafkaMessage {
    topic: message.topic().to_string(),
    partition: message.partition(),
    offset: message.offset(),
    key: message.key().map(|k| k.to_vec()),
    payload: message.payload().unwrap_or_default().to_vec(),
    timestamp: message.timestamp().to_millis(),
    headers,
  }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
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

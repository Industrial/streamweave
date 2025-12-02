#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use super::kafka_consumer::KafkaConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use async_trait::async_trait;
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use futures::StreamExt;
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use rdkafka::{
  config::ClientConfig,
  producer::{FutureProducer, FutureRecord},
  util::Timeout,
};
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use serde::Serialize;
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use std::time::Duration;
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use tracing::{error, warn};

#[async_trait]
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
impl<T> Consumer for KafkaConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  /// Consumes a stream and sends each item as a message to a Kafka topic.
  ///
  /// # Error Handling
  ///
  /// - If the Kafka producer cannot be created, an error is logged and no data is sent.
  /// - If an item cannot be serialized or sent, the error strategy determines the action.
  async fn consume(&mut self, input: Self::InputStream) {
    let kafka_config = self.kafka_config.clone();
    let component_name = if self.config.name.is_empty() {
      "kafka_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let error_strategy = self.config.error_strategy.clone();

    // Build Kafka producer configuration
    let mut client_config = ClientConfig::new();
    client_config.set("bootstrap.servers", &kafka_config.bootstrap_servers);
    client_config.set("acks", &kafka_config.acks);
    client_config.set("retries", kafka_config.retries.to_string());
    client_config.set("batch.size", kafka_config.batch_size.to_string());
    client_config.set("linger.ms", kafka_config.linger_ms.to_string());
    client_config.set(
      "max.request.size",
      kafka_config.max_request_size.to_string(),
    );

    if kafka_config.compression_type != "none" {
      client_config.set("compression.type", &kafka_config.compression_type);
    }

    if let Some(ref client_id) = kafka_config.client_id {
      client_config.set("client.id", client_id);
    }

    // Apply custom properties
    for (key, value) in &kafka_config.custom_properties {
      client_config.set(key, value);
    }

    // Create producer
    let producer: FutureProducer = match client_config.create() {
      Ok(p) => p,
      Err(e) => {
        error!(
          component = %component_name,
          error = %e,
          "Failed to create Kafka producer"
        );
        return;
      }
    };

    let topic = kafka_config.topic.clone();
    let mut input = std::pin::pin!(input);

    // Send messages
    while let Some(item) = input.next().await {
      // Serialize the item
      let payload = match serde_json::to_vec(&item) {
        Ok(p) => p,
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

      // Send to Kafka
      let record: FutureRecord<(), Vec<u8>> = FutureRecord::to(&topic).payload(&payload);

      match producer
        .send(record, Timeout::After(Duration::from_secs(5)))
        .await
      {
        Ok(_) => {
          // Message sent successfully
        }
        Err((e, _message)) => {
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
                topic = %topic,
                error = %error,
                "Stopping due to Kafka send error"
              );
              break;
            }
            ErrorAction::Skip => {
              warn!(
                component = %component_name,
                topic = %topic,
                error = %error,
                "Skipping item due to Kafka send error"
              );
              continue;
            }
            ErrorAction::Retry => {
              warn!(
                component = %component_name,
                topic = %topic,
                error = %error,
                "Retry not fully supported for Kafka send errors, skipping"
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

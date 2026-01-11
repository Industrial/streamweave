//! # Kafka Publish Transformer
//!
//! Transformer that publishes serializable data to Kafka topics while passing the
//! same data through to the output stream. This enables publishing items to Kafka
//! for downstream consumption while continuing the main pipeline flow.
//!
//! ## Overview
//!
//! The Kafka Publish Transformer provides:
//!
//! - **Kafka Publishing**: Publishes items to Kafka topics using rdkafka
//! - **Pass-Through**: Outputs the same items that were published
//! - **Serialization**: Automatically serializes items to JSON for Kafka
//! - **Connection Management**: Manages Kafka producer connection lifecycle
//! - **Error Handling**: Configurable error strategies for publish failures
//!
//! ## Input/Output
//!
//! - **Input**: `Message<T>` - Serializable items to publish to Kafka
//! - **Output**: `Message<T>` - The same items (pass-through)
//!
//! ## Use Cases
//!
//! - **Event Streaming**: Publish events to Kafka for real-time processing
//! - **Fan-Out**: Distribute items to multiple Kafka consumers
//! - **Data Pipeline**: Integrate with Kafka-based data pipelines
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::transformers::KafkaPublishTransformer;
//! use streamweave::consumers::KafkaProducerConfig;
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct Event {
//!     id: u32,
//!     message: String,
//! }
//!
//! let config = KafkaProducerConfig::default()
//!   .with_bootstrap_servers("localhost:9092")
//!   .with_topic("events");
//! let transformer = KafkaPublishTransformer::<Event>::new(config);
//! // Input: [Event, ...]
//! // Publishes to Kafka and outputs: [Event, ...]
//! ```

use crate::consumers::KafkaProducerConfig;
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use rdkafka::{
  config::ClientConfig,
  producer::{FutureProducer, FutureRecord},
  util::Timeout,
};
use serde::Serialize;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, warn};

/// A transformer that publishes data to Kafka while passing data through.
///
/// Each input item is serialized and published to a Kafka topic,
/// and then the same item is output, enabling publishing to Kafka and continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::KafkaPublishTransformer;
/// use streamweave::consumers::KafkaProducerConfig;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let config = KafkaProducerConfig::default()
///   .with_bootstrap_servers("localhost:9092")
///   .with_topic("events");
/// let transformer = KafkaPublishTransformer::<Event>::new(config);
/// // Input: [Event, ...]
/// // Publishes to Kafka and outputs: [Event, ...]
/// ```
pub struct KafkaPublishTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Kafka producer configuration.
  kafka_config: KafkaProducerConfig,
  /// Transformer configuration.
  config: TransformerConfig<T>,
  /// Producer handle (shared across stream items).
  producer: Arc<Mutex<Option<FutureProducer>>>,
}

impl<T> KafkaPublishTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `KafkaPublishTransformer` with the given Kafka configuration.
  ///
  /// # Arguments
  ///
  /// * `kafka_config` - Kafka producer configuration.
  #[must_use]
  pub fn new(kafka_config: KafkaProducerConfig) -> Self {
    Self {
      kafka_config,
      config: TransformerConfig::default(),
      producer: Arc::new(Mutex::new(None)),
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

  /// Returns the Kafka producer configuration.
  #[must_use]
  pub fn kafka_config(&self) -> &KafkaProducerConfig {
    &self.kafka_config
  }
}

impl<T> Clone for KafkaPublishTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      kafka_config: self.kafka_config.clone(),
      config: self.config.clone(),
      producer: Arc::clone(&self.producer),
    }
  }
}

impl<T> Input for KafkaPublishTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for KafkaPublishTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for KafkaPublishTransformer<T>
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
      .unwrap_or_else(|| "kafka_publish_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let kafka_config = self.kafka_config.clone();
    let producer = Arc::clone(&self.producer);
    let topic = kafka_config.topic.clone();

    Box::pin(async_stream::stream! {
      // Initialize producer on first item
      {
        let mut producer_guard = producer.lock().await;
        if producer_guard.is_none() {
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
          match client_config.create() {
            Ok(p) => {
              *producer_guard = Some(p);
            }
            Err(e) => {
              error!(
                component = %component_name,
                error = %e,
                "Failed to create Kafka producer"
              );
              return;
            }
          }
        }
      }

      let mut input_stream = input;
      while let Some(item) = input_stream.next().await {
        // Serialize the item
        let payload = match serde_json::to_vec(&item) {
          Ok(p) => p,
          Err(e) => {
            let stream_error = StreamError::new(
              Box::new(e),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(item.clone()),
                component_name: component_name.clone(),
                component_type: std::any::type_name::<KafkaPublishTransformer<T>>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<KafkaPublishTransformer<T>>().to_string(),
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

        // Send to Kafka
        {
          let producer_guard = producer.lock().await;
          if let Some(ref p) = *producer_guard {
            let record: FutureRecord<(), Vec<u8>> = FutureRecord::to(&topic).payload(&payload);

            match p
              .send(record, Timeout::After(Duration::from_secs(5)))
              .await
            {
              Ok(_) => {
                // Message sent successfully
              }
              Err((e, _message)) => {
                let stream_error = StreamError::new(
                  Box::new(e),
                  ErrorContext {
                    timestamp: chrono::Utc::now(),
                    item: Some(item.clone()),
                    component_name: component_name.clone(),
                    component_type: std::any::type_name::<KafkaPublishTransformer<T>>().to_string(),
                  },
                  ComponentInfo {
                    name: component_name.clone(),
                    type_name: std::any::type_name::<KafkaPublishTransformer<T>>().to_string(),
                  },
                );

                match handle_error_strategy(&error_strategy, &stream_error) {
                  ErrorAction::Stop => {
                    error!(
                      component = %component_name,
                      topic = %topic,
                      error = %stream_error,
                      "Stopping due to Kafka send error"
                    );
                    return;
                  }
                  ErrorAction::Skip => {
                    warn!(
                      component = %component_name,
                      topic = %topic,
                      error = %stream_error,
                      "Skipping item due to Kafka send error"
                    );
                    continue;
                  }
                  ErrorAction::Retry => {
                    warn!(
                      component = %component_name,
                      topic = %topic,
                      error = %stream_error,
                      "Retry not supported for Kafka send errors, skipping"
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
        .unwrap_or_else(|| "kafka_publish_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "kafka_publish_transformer".to_string()),
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

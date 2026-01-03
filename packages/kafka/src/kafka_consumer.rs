#[cfg(feature = "kafka")]
use async_trait::async_trait;
#[cfg(feature = "kafka")]
use futures::{Stream, StreamExt};
#[cfg(feature = "kafka")]
use rdkafka::{
  config::ClientConfig,
  producer::{FutureProducer, FutureRecord},
  util::Timeout,
};
use serde::Serialize;
use std::collections::HashMap;
#[cfg(feature = "kafka")]
use std::pin::Pin;
#[cfg(feature = "kafka")]
use std::time::Duration;
use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave::{Consumer, ConsumerConfig, Input};
#[cfg(feature = "kafka")]
use tracing::{error, warn};

/// Configuration for Kafka producer behavior.
#[derive(Debug, Clone)]
pub struct KafkaProducerConfig {
  /// Bootstrap servers (comma-separated list of broker addresses).
  pub bootstrap_servers: String,
  /// Topic to produce to.
  pub topic: String,
  /// Client ID for the producer.
  pub client_id: Option<String>,
  /// Acks configuration ("0", "1", or "all").
  pub acks: String,
  /// Maximum number of retries.
  pub retries: u32,
  /// Batch size in bytes.
  pub batch_size: usize,
  /// Linger time in milliseconds (wait time before sending batch).
  pub linger_ms: u64,
  /// Maximum request size in bytes.
  pub max_request_size: usize,
  /// Compression type ("none", "gzip", "snappy", "lz4", "zstd").
  pub compression_type: String,
  /// Additional custom configuration properties.
  pub custom_properties: HashMap<String, String>,
}

impl Default for KafkaProducerConfig {
  fn default() -> Self {
    Self {
      bootstrap_servers: "localhost:9092".to_string(),
      topic: String::new(),
      client_id: None,
      acks: "all".to_string(),
      retries: 3,
      batch_size: 16384, // 16KB
      linger_ms: 0,
      max_request_size: 1048576, // 1MB
      compression_type: "none".to_string(),
      custom_properties: HashMap::new(),
    }
  }
}

impl KafkaProducerConfig {
  /// Sets the bootstrap servers.
  #[must_use]
  pub fn with_bootstrap_servers(mut self, servers: impl Into<String>) -> Self {
    self.bootstrap_servers = servers.into();
    self
  }

  /// Sets the topic to produce to.
  #[must_use]
  pub fn with_topic(mut self, topic: impl Into<String>) -> Self {
    self.topic = topic.into();
    self
  }

  /// Sets the client ID.
  #[must_use]
  pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
    self.client_id = Some(client_id.into());
    self
  }

  /// Sets the acks configuration.
  #[must_use]
  pub fn with_acks(mut self, acks: impl Into<String>) -> Self {
    self.acks = acks.into();
    self
  }

  /// Sets the number of retries.
  #[must_use]
  pub fn with_retries(mut self, retries: u32) -> Self {
    self.retries = retries;
    self
  }

  /// Sets the batch size.
  #[must_use]
  pub fn with_batch_size(mut self, size: usize) -> Self {
    self.batch_size = size;
    self
  }

  /// Sets the linger time.
  #[must_use]
  pub fn with_linger_ms(mut self, ms: u64) -> Self {
    self.linger_ms = ms;
    self
  }

  /// Sets the compression type.
  #[must_use]
  pub fn with_compression_type(mut self, compression: impl Into<String>) -> Self {
    self.compression_type = compression.into();
    self
  }

  /// Sets a custom property.
  #[must_use]
  pub fn with_custom_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.custom_properties.insert(key.into(), value.into());
    self
  }
}

/// A consumer that produces messages to Kafka topics.
///
/// This consumer serializes input elements and sends them to a Kafka topic.
/// It supports batching, retries, and various Kafka producer configurations.
///
/// # Example
///
/// ```ignore
/// use streamweave_kafka::KafkaConsumer;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let consumer = KafkaConsumer::<Event>::new(
///     KafkaProducerConfig::default()
///         .with_bootstrap_servers("localhost:9092")
///         .with_topic("my-topic")
/// );
/// ```
pub struct KafkaConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Consumer configuration.
  pub config: ConsumerConfig<T>,
  /// Kafka producer-specific configuration.
  pub kafka_config: KafkaProducerConfig,
}

impl<T> KafkaConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new Kafka consumer with the given configuration.
  #[must_use]
  pub fn new(kafka_config: KafkaProducerConfig) -> Self {
    Self {
      config: ConsumerConfig::default(),
      kafka_config,
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

  /// Returns the Kafka producer configuration.
  #[must_use]
  pub fn kafka_config(&self) -> &KafkaProducerConfig {
    &self.kafka_config
  }
}

impl<T> Clone for KafkaConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      kafka_config: self.kafka_config.clone(),
    }
  }
}

impl<T> Input for KafkaConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

#[async_trait]
#[cfg(feature = "kafka")]
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

#[cfg(feature = "kafka")]
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

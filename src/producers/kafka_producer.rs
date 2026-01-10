use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use async_stream::stream;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, warn};

use rdkafka::{
  ClientContext, Statistics,
  config::{ClientConfig, RDKafkaLogLevel},
  consumer::stream_consumer::StreamConsumer,
  consumer::{Consumer, ConsumerContext},
  message::{BorrowedMessage, Headers, Message},
};

/// Configuration for Kafka consumer behavior.
#[derive(Debug, Clone)]
pub struct KafkaConsumerConfig {
  /// Bootstrap servers (comma-separated list of broker addresses).
  pub bootstrap_servers: String,
  /// Consumer group ID.
  pub group_id: Option<String>,
  /// Topics to consume from.
  pub topics: Vec<String>,
  /// Auto offset reset strategy ("earliest" or "latest").
  pub auto_offset_reset: String,
  /// Enable auto commit of offsets.
  pub enable_auto_commit: bool,
  /// Auto commit interval in milliseconds.
  pub auto_commit_interval_ms: u64,
  /// Session timeout in milliseconds.
  pub session_timeout_ms: u64,
  /// Maximum poll interval in milliseconds.
  pub max_poll_interval_ms: u64,
  /// Maximum number of bytes to fetch per request.
  pub fetch_max_bytes: usize,
  /// Maximum wait time for fetch requests in milliseconds.
  pub fetch_wait_max_ms: u64,
  /// Additional custom configuration properties.
  pub custom_properties: HashMap<String, String>,
}

impl Default for KafkaConsumerConfig {
  fn default() -> Self {
    Self {
      bootstrap_servers: "localhost:9092".to_string(),
      group_id: None,
      topics: Vec::new(),
      auto_offset_reset: "earliest".to_string(),
      enable_auto_commit: true,
      auto_commit_interval_ms: 5000,
      session_timeout_ms: 30000,
      max_poll_interval_ms: 300000,
      fetch_max_bytes: 1048576, // 1MB
      fetch_wait_max_ms: 500,
      custom_properties: HashMap::new(),
    }
  }
}

impl KafkaConsumerConfig {
  /// Sets the bootstrap servers.
  #[must_use]
  pub fn with_bootstrap_servers(mut self, servers: impl Into<String>) -> Self {
    self.bootstrap_servers = servers.into();
    self
  }

  /// Sets the consumer group ID.
  #[must_use]
  pub fn with_group_id(mut self, group_id: impl Into<String>) -> Self {
    self.group_id = Some(group_id.into());
    self
  }

  /// Adds a topic to consume from.
  #[must_use]
  pub fn with_topic(mut self, topic: impl Into<String>) -> Self {
    self.topics.push(topic.into());
    self
  }

  /// Sets multiple topics to consume from.
  #[must_use]
  pub fn with_topics(mut self, topics: Vec<String>) -> Self {
    self.topics = topics;
    self
  }

  /// Sets the auto offset reset strategy.
  #[must_use]
  pub fn with_auto_offset_reset(mut self, reset: impl Into<String>) -> Self {
    self.auto_offset_reset = reset.into();
    self
  }

  /// Sets whether to enable auto commit.
  #[must_use]
  pub fn with_enable_auto_commit(mut self, enable: bool) -> Self {
    self.enable_auto_commit = enable;
    self
  }

  /// Sets a custom property.
  #[must_use]
  pub fn with_custom_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.custom_properties.insert(key.into(), value.into());
    self
  }

  /// Sets the auto commit interval in milliseconds.
  #[must_use]
  pub fn with_auto_commit_interval_ms(mut self, interval_ms: u64) -> Self {
    self.auto_commit_interval_ms = interval_ms;
    self
  }
}

/// A message received from Kafka.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaMessage {
  /// Topic name.
  pub topic: String,
  /// Partition number.
  pub partition: i32,
  /// Offset.
  pub offset: i64,
  /// Message key (if any).
  pub key: Option<Vec<u8>>,
  /// Message payload.
  pub payload: Vec<u8>,
  /// Message timestamp.
  pub timestamp: Option<i64>,
  /// Message headers.
  pub headers: HashMap<String, Vec<u8>>,
}

/// A producer that consumes messages from Kafka topics.
///
/// This producer reads messages from one or more Kafka topics and yields
/// them as a stream. It supports consumer groups, partition assignment,
/// and offset management.
///
/// # Example
///
/// ```ignore
/// use streamweave_kafka::KafkaProducer;
///
/// let producer = KafkaProducer::new(
///     KafkaConsumerConfig::default()
///         .with_bootstrap_servers("localhost:9092")
///         .with_group_id("my-consumer-group")
///         .with_topic("my-topic")
/// );
/// ```
pub struct KafkaProducer {
  /// Kafka consumer configuration.
  pub config: ProducerConfig<KafkaMessage>,
  /// Kafka consumer-specific configuration.
  pub kafka_config: KafkaConsumerConfig,
}

impl KafkaProducer {
  /// Creates a new Kafka producer with the given configuration.
  #[must_use]
  pub fn new(kafka_config: KafkaConsumerConfig) -> Self {
    Self {
      config: ProducerConfig::default(),
      kafka_config,
    }
  }

  /// Sets the error strategy for the producer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<KafkaMessage>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the producer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the Kafka consumer configuration.
  #[must_use]
  pub fn kafka_config(&self) -> &KafkaConsumerConfig {
    &self.kafka_config
  }
}

impl Clone for KafkaProducer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      kafka_config: self.kafka_config.clone(),
    }
  }
}

impl Output for KafkaProducer {
  type Output = KafkaMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Producer for KafkaProducer {
  type OutputPorts = (KafkaMessage,);

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
        kafka_config.auto_commit_interval_ms.to_string(),
      );

      // Set session and poll timeouts
      client_config.set("session.timeout.ms", kafka_config.session_timeout_ms.to_string());
      client_config.set(
        "max.poll.interval.ms",
        kafka_config.max_poll_interval_ms.to_string(),
      );

      // Set fetch settings
      client_config.set("fetch.max.bytes", kafka_config.fetch_max_bytes.to_string());
      client_config.set("fetch.wait.max.ms", kafka_config.fetch_wait_max_ms.to_string());

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

struct StreamWeaveConsumerContext;

impl ClientContext for StreamWeaveConsumerContext {
  fn stats(&self, _statistics: Statistics) {
    // Statistics can be logged or collected here if needed
  }
}

impl ConsumerContext for StreamWeaveConsumerContext {}

fn convert_message(message: &BorrowedMessage<'_>) -> KafkaMessage {
  let mut headers = std::collections::HashMap::new();
  if let Some(message_headers) = message.headers() {
    for header in message_headers.iter() {
      if let (key, Some(value)) = (header.key, header.value) {
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

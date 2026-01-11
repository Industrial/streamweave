//! Redis Streams producer for reading messages from Redis Streams.
//!
//! This module provides [`RedisProducer`], a producer that reads messages from Redis Streams
//! and produces them as a stream. It supports consumer groups, message acknowledgment, and
//! pending message tracking, making it ideal for distributed message processing.
//!
//! # Overview
//!
//! [`RedisProducer`] connects to Redis and reads messages from Redis Streams. It supports
//! both simple stream reading and consumer groups for distributed processing. Messages are
//! yielded as [`RedisMessage`] objects containing the stream name, message ID, and field data.
//!
//! # Key Concepts
//!
//! - **Redis Streams**: Reads from Redis Streams (a Redis data structure for message streams)
//! - **Consumer Groups**: Supports Redis consumer groups for distributed processing
//! - **Message Acknowledgment**: Can automatically acknowledge messages in consumer groups
//! - **Blocking Reads**: Supports blocking reads with configurable timeout
//! - **Async Redis Client**: Uses async Redis client for efficient I/O
//! - **Error Handling**: Configurable error strategies for connection and read failures
//!
//! # Core Types
//!
//! - **[`RedisProducer`]**: Producer that reads messages from Redis Streams
//! - **[`RedisConsumerConfig`]**: Configuration for Redis Streams reading behavior
//! - **[`RedisMessage`]**: Message structure containing stream name, ID, and fields
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::producers::{RedisProducer, RedisConsumerConfig};
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that reads from Redis Streams
//! let config = RedisConsumerConfig::default()
//!     .with_connection_url("redis://localhost:6379")
//!     .with_stream("mystream")
//!     .with_start_id("0");  // Start from beginning
//! let producer = RedisProducer::new(config);
//!
//! // Use in a pipeline
//! let pipeline = PipelineBuilder::new()
//!     .producer(producer)
//!     .transformer(/* ... */)
//!     .consumer(/* ... */);
//! # Ok(())
//! # }
//! ```
//!
//! ## With Consumer Groups
//!
//! ```rust
//! use streamweave::producers::{RedisProducer, RedisConsumerConfig};
//!
//! // Create a producer with consumer group for distributed processing
//! let config = RedisConsumerConfig::default()
//!     .with_connection_url("redis://localhost:6379")
//!     .with_stream("mystream")
//!     .with_group("my-group")
//!     .with_consumer("consumer-1")
//!     .with_auto_ack(true);  // Auto-acknowledge messages
//! let producer = RedisProducer::new(config);
//! ```
//!
//! # Design Decisions
//!
//! - **Consumer Groups**: Supports Redis consumer groups for scalable, distributed message
//!   processing with load balancing and fault tolerance
//! - **Blocking Reads**: Uses blocking reads with timeout for efficient polling
//! - **Message Acknowledgment**: Supports automatic acknowledgment for reliable processing
//! - **Async Client**: Uses async Redis client for efficient, non-blocking I/O
//! - **Field Representation**: Converts Redis stream fields to HashMap for flexible access
//!
//! # Integration with StreamWeave
//!
//! [`RedisProducer`] implements the [`Producer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ProducerConfig`].

//! # Redis Streams Producer
//!
//! Producer for reading messages from Redis Streams in StreamWeave pipelines.
//!
//! This module provides [`RedisProducer`], a producer that reads messages from Redis Streams
//! and yields them as a stream. It supports consumer groups, message acknowledgment, pending
//! message tracking, and error handling strategies.
//!
//! # Overview
//!
//! [`RedisProducer`] reads messages from Redis Streams, which is Redis's log-like data structure
//! that models a log file data structure. It supports consumer groups for distributed message
//! processing, message acknowledgment, and various reading patterns (blocking, non-blocking,
//! from beginning, from new messages).
//!
//! # Key Concepts
//!
//! ## Redis Streams
//!
//! Redis Streams is a data structure that acts like a log file. Messages are appended to the
//! stream with unique IDs, and consumers can read from any position in the stream. Streams
//! support:
//!
//! - **Message IDs**: Each message has a unique ID (timestamp-based)
//! - **Consumer Groups**: Multiple consumers can read from the same stream
//! - **Message Acknowledgment**: Consumers can acknowledge processed messages
//! - **Pending Messages**: Track unacknowledged messages
//!
//! ## Consumer Groups
//!
//! Consumer groups enable distributed message processing where multiple consumers read from
//! the same stream, with each message delivered to only one consumer in the group. This enables
//! load balancing and parallel processing.
//!
//! - **Group Name**: Identifies the consumer group
//! - **Consumer Name**: Identifies the consumer within the group
//! - **Message Distribution**: Messages are distributed across consumers in the group
//! - **Acknowledgment**: Messages must be acknowledged after processing
//!
//! ## Message Reading Patterns
//!
//! - **From Beginning**: Read from ID "0" to get all messages
//! - **New Messages**: Use ">" to read only new messages in consumer groups
//! - **From ID**: Read from a specific message ID
//! - **Blocking**: Wait for new messages (block_ms > 0)
//! - **Non-Blocking**: Return immediately (block_ms = 0)
//!
//! # Core Types
//!
//! - **[`RedisProducer`]**: Producer that reads messages from Redis Streams
//! - **[`RedisMessage`]**: Message structure containing stream name, ID, and fields
//! - **[`RedisConsumerConfig`]**: Configuration for Redis Streams consumer behavior
//! - **[`RedisProducerConfig`]**: Configuration for Redis Streams producer behavior
//!
//! # Quick Start
//!
//! ## Basic Usage (Simple Read)
//!
//! ```rust,no_run
//! use streamweave::producers::{RedisProducer, RedisConsumerConfig};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that reads from a Redis stream
//! let mut producer = RedisProducer::new(
//!     RedisConsumerConfig::default()
//!         .with_connection_url("redis://localhost:6379")
//!         .with_stream("mystream")
//!         .with_start_id("0")  // Read from beginning
//! );
//!
//! // Generate the stream
//! let mut stream = producer.produce();
//!
//! // Process messages
//! while let Some(message) = stream.next().await {
//!     println!("Received message: {:?}", message);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Consumer Group Usage
//!
//! ```rust,no_run
//! use streamweave::producers::{RedisProducer, RedisConsumerConfig};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer with consumer group
//! let mut producer = RedisProducer::new(
//!     RedisConsumerConfig::default()
//!         .with_connection_url("redis://localhost:6379")
//!         .with_stream("mystream")
//!         .with_group("my-group")
//!         .with_consumer("consumer-1")
//!         .with_auto_ack(true)  // Auto-acknowledge messages
//! );
//!
//! let mut stream = producer.produce();
//!
//! // Process messages (each message goes to only one consumer in the group)
//! while let Some(message) = stream.next().await {
//!     println!("Processing message: {:?}", message);
//!     // Message is auto-acknowledged
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Blocking Read (Wait for New Messages)
//!
//! ```rust,no_run
//! use streamweave::producers::{RedisProducer, RedisConsumerConfig};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that blocks waiting for new messages
//! let mut producer = RedisProducer::new(
//!     RedisConsumerConfig::default()
//!         .with_connection_url("redis://localhost:6379")
//!         .with_stream("mystream")
//!         .with_start_id("$")  // Read only new messages
//!         .with_block_ms(5000)  // Block for up to 5 seconds
//! );
//!
//! let mut stream = producer.produce();
//!
//! // Will block waiting for new messages
//! while let Some(message) = stream.next().await {
//!     println!("New message: {:?}", message);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Error Handling
//!
//! ```rust,no_run
//! use streamweave::producers::{RedisProducer, RedisConsumerConfig};
//! use streamweave::ErrorStrategy;
//!
//! // Create a producer with error handling strategy
//! let producer = RedisProducer::new(
//!     RedisConsumerConfig::default()
//!         .with_connection_url("redis://localhost:6379")
//!         .with_stream("mystream")
//! )
//! .with_error_strategy(ErrorStrategy::Retry(3))  // Retry up to 3 times
//! .with_name("redis-reader".to_string());
//! ```
//!
//! # Design Decisions
//!
//! ## Async Redis Client
//!
//! Uses the `redis` crate's async client for non-blocking I/O. This enables efficient
//! concurrent processing and integrates seamlessly with Tokio's async runtime.
//!
//! ## Connection Management
//!
//! Creates a single connection manager per producer that is reused throughout the stream's
//! lifetime. This ensures efficient connection usage and avoids connection overhead.
//!
//! ## Consumer Group Support
//!
//! Fully supports Redis consumer groups for distributed message processing. Automatically
//! creates consumer groups if they don't exist and handles message distribution across
//! consumers in the group.
//!
//! ## Message Acknowledgment
//!
//! Supports automatic message acknowledgment when using consumer groups. When `auto_ack` is
//! enabled, messages are automatically acknowledged after being yielded. Manual acknowledgment
//! can be implemented by processing messages and using Redis commands directly.
//!
//! ## Error Handling Strategies
//!
//! Supports all standard StreamWeave error handling strategies (Stop, Skip, Retry, Custom).
//! This allows flexible error handling for connection errors, read errors, and other failures.
//!
//! ## Blocking vs Non-Blocking
//!
//! Supports both blocking and non-blocking reads. Blocking reads (block_ms > 0) wait for new
//! messages, making them efficient for real-time processing. Non-blocking reads (block_ms = 0)
//! return immediately, useful for polling patterns.
//!
//! # Integration with StreamWeave
//!
//! [`RedisProducer`] integrates seamlessly with StreamWeave's pipeline and graph systems:
//!
//! - **Pipeline API**: Use in pipelines for Redis Streams integration
//! - **Graph API**: Wrap in [`crate::graph::nodes::ProducerNode`] for graph-based execution
//! - **Error Handling**: Supports standard error handling strategies
//! - **Configuration**: Supports configuration via [`ProducerConfig`] and [`RedisConsumerConfig`]
//! - **Message Model**: Messages can be wrapped in `Message<T>` for traceability
//!
//! # Common Patterns
//!
//! ## Distributed Processing with Consumer Groups
//!
//! Use consumer groups to distribute message processing across multiple instances:
//!
//! ```rust,no_run
//! use streamweave::producers::{RedisProducer, RedisConsumerConfig};
//!
//! // Instance 1
//! let producer1 = RedisProducer::new(
//!     RedisConsumerConfig::default()
//!         .with_stream("mystream")
//!         .with_group("my-group")
//!         .with_consumer("worker-1")
//! );
//!
//! // Instance 2
//! let producer2 = RedisProducer::new(
//!     RedisConsumerConfig::default()
//!         .with_stream("mystream")
//!         .with_group("my-group")
//!         .with_consumer("worker-2")
//! );
//!
//! // Both instances read from the same stream, messages are distributed
//! ```
//!
//! ## Processing from Beginning
//!
//! Read all messages from the beginning of the stream:
//!
//! ```rust,no_run
//! use streamweave::producers::{RedisProducer, RedisConsumerConfig};
//!
//! let producer = RedisProducer::new(
//!     RedisConsumerConfig::default()
//!         .with_stream("mystream")
//!         .with_start_id("0")  // Start from beginning
//! );
//! ```
//!
//! ## Processing Only New Messages
//!
//! Read only new messages that arrive after the producer starts:
//!
//! ```rust,no_run
//! use streamweave::producers::{RedisProducer, RedisConsumerConfig};
//!
//! let producer = RedisProducer::new(
//!     RedisConsumerConfig::default()
//!         .with_stream("mystream")
//!         .with_start_id("$")  // Only new messages
//!         .with_block_ms(1000)  // Block waiting for new messages
//! );
//! ```
//!
//! ## Manual Message Acknowledgment
//!
//! Process messages and acknowledge them manually (advanced):
//!
//! ```rust,no_run
//! use streamweave::producers::{RedisProducer, RedisConsumerConfig};
//!
//! let producer = RedisProducer::new(
//!     RedisConsumerConfig::default()
//!         .with_stream("mystream")
//!         .with_group("my-group")
//!         .with_consumer("consumer-1")
//!         .with_auto_ack(false)  // Manual acknowledgment
//! );
//!
//! // Process messages and use Redis commands to acknowledge
//! // (requires access to Redis connection)
//! ```
//!
//! # Redis Streams Concepts
//!
//! ## Message IDs
//!
//! Messages in Redis Streams have unique IDs based on timestamps (milliseconds-s sequence).
//! IDs are used for:
//!
//! - **Ordering**: Messages are ordered by ID
//! - **Position Tracking**: Track reading position in the stream
//! - **Consumer Groups**: Track processed messages per consumer
//!
//! ## Stream Length
//!
//! Streams can have a maximum length to limit memory usage. Use `maxlen` in producer config
//! to automatically trim streams when writing.
//!
//! ## Pending Messages
//!
//! When using consumer groups, messages that are read but not acknowledged are "pending".
//! Pending messages can be retrieved and processed if a consumer fails.

use crate::error::ErrorStrategy;
use crate::{Output, Producer, ProducerConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for writing to Redis Streams (producing).
#[derive(Debug, Clone)]
pub struct RedisProducerConfig {
  /// Redis connection URL (e.g., "redis://localhost:6379").
  pub connection_url: String,
  /// Stream name to write to.
  pub stream: String,
  /// Maximum length of the stream (None for unlimited).
  pub maxlen: Option<usize>,
  /// Whether to use approximate maxlen (more efficient for large streams).
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

  /// Sets whether to use approximate maxlen.
  #[must_use]
  pub fn with_approximate_maxlen(mut self, approximate: bool) -> Self {
    self.approximate_maxlen = approximate;
    self
  }
}

/// Configuration for Redis Streams consumer behavior.
#[derive(Debug, Clone)]
pub struct RedisConsumerConfig {
  /// Redis connection URL (e.g., "redis://localhost:6379").
  pub connection_url: String,
  /// Stream name to consume from.
  pub stream: String,
  /// Consumer group name (optional, enables consumer groups).
  pub group: Option<String>,
  /// Consumer name (required if using consumer groups).
  pub consumer: Option<String>,
  /// Starting ID for reading (use "0" for beginning, "$" for new messages).
  pub start_id: String,
  /// Block time in milliseconds (0 for non-blocking).
  pub block_ms: u64,
  /// Count of messages to read per call.
  pub count: Option<usize>,
  /// Whether to acknowledge messages automatically.
  pub auto_ack: bool,
}

impl Default for RedisConsumerConfig {
  fn default() -> Self {
    Self {
      connection_url: "redis://localhost:6379".to_string(),
      stream: String::new(),
      group: None,
      consumer: None,
      start_id: "0".to_string(),
      block_ms: 1000,
      count: None,
      auto_ack: false,
    }
  }
}

impl RedisConsumerConfig {
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

  /// Sets the consumer group.
  #[must_use]
  pub fn with_group(mut self, group: impl Into<String>) -> Self {
    self.group = Some(group.into());
    self
  }

  /// Sets the consumer name.
  #[must_use]
  pub fn with_consumer(mut self, consumer: impl Into<String>) -> Self {
    self.consumer = Some(consumer.into());
    self
  }

  /// Sets the starting ID.
  #[must_use]
  pub fn with_start_id(mut self, id: impl Into<String>) -> Self {
    self.start_id = id.into();
    self
  }

  /// Sets the block time in milliseconds.
  #[must_use]
  pub fn with_block_ms(mut self, ms: u64) -> Self {
    self.block_ms = ms;
    self
  }

  /// Sets the count of messages per read.
  #[must_use]
  pub fn with_count(mut self, count: usize) -> Self {
    self.count = Some(count);
    self
  }

  /// Sets whether to auto-acknowledge messages.
  #[must_use]
  pub fn with_auto_ack(mut self, auto_ack: bool) -> Self {
    self.auto_ack = auto_ack;
    self
  }
}

/// A message received from Redis Streams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisMessage {
  /// Stream name.
  pub stream: String,
  /// Message ID.
  pub id: String,
  /// Message fields as key-value pairs.
  pub fields: HashMap<String, String>,
}

/// A producer that consumes messages from Redis Streams.
///
/// This producer reads messages from a Redis stream and yields them as
/// a stream. It supports consumer groups, message acknowledgment, and
/// pending message tracking.
///
/// # Example
///
/// ```ignore
/// use crate::producers::{RedisProducer, RedisConsumerConfig};
///
/// let producer = RedisProducer::new(
///     RedisConsumerConfig::default()
///         .with_connection_url("redis://localhost:6379")
///         .with_stream("mystream")
///         .with_group("my-group")
///         .with_consumer("consumer-1")
/// );
/// ```
pub struct RedisProducer {
  /// Producer configuration.
  pub config: ProducerConfig<RedisMessage>,
  /// Redis Streams consumer-specific configuration.
  pub redis_config: RedisConsumerConfig,
}

impl RedisProducer {
  /// Creates a new Redis Streams producer with the given configuration.
  #[must_use]
  pub fn new(redis_config: RedisConsumerConfig) -> Self {
    Self {
      config: ProducerConfig::default(),
      redis_config,
    }
  }

  /// Sets the error strategy for the producer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<RedisMessage>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the producer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the Redis Streams consumer configuration.
  #[must_use]
  pub fn redis_config(&self) -> &RedisConsumerConfig {
    &self.redis_config
  }
}

impl Clone for RedisProducer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      redis_config: self.redis_config.clone(),
    }
  }
}

// Output trait implementation
use futures::Stream;
use std::pin::Pin;

impl Output for RedisProducer {
  type Output = RedisMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

// Producer trait implementation
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};
use async_stream::stream;
use async_trait::async_trait;
use redis::{
  AsyncCommands, Client, RedisResult,
  aio::ConnectionManager,
  streams::{StreamReadOptions, StreamReadReply},
};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, warn};

#[async_trait]
#[allow(clippy::collapsible_if)]
impl Producer for RedisProducer {
  type OutputPorts = (RedisMessage,);

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

pub(crate) fn handle_error_strategy<T>(
  strategy: &crate::error::ErrorStrategy<T>,
  error: &StreamError<T>,
) -> ErrorAction
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
  match strategy {
    crate::error::ErrorStrategy::Stop => ErrorAction::Stop,
    crate::error::ErrorStrategy::Skip => ErrorAction::Skip,
    crate::error::ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    crate::error::ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}

//! Kafka publish node for publishing data to Kafka while passing data through.
//!
//! This module provides [`KafkaPublish`], a graph node that publishes data to Kafka
//! topics while passing the same data through to the output. It takes serializable
//! data as input, publishes it to a Kafka topic, and outputs the same data, enabling
//! publishing to Kafka and continuing processing. It wraps [`KafkaPublishTransformer`]
//! for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`KafkaPublish`] is useful for publishing intermediate results to Kafka topics
//! while continuing processing in graph-based pipelines. Unlike consumers, it passes
//! data through, making it ideal for checkpointing data at intermediate stages or
//! logging Kafka publishes.
//!
//! # Key Concepts
//!
//! - **Pass-Through Operation**: Publishes data to Kafka while passing it through to output
//! - **Kafka Integration**: Publishes messages to Kafka topics using Kafka producer
//! - **Intermediate Results**: Enables publishing intermediate results without
//!   interrupting the pipeline
//! - **Transformer Wrapper**: Wraps `KafkaPublishTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`KafkaPublish<T>`]**: Node that publishes data to Kafka while passing data through
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::KafkaPublish;
//! use streamweave::consumers::KafkaProducerConfig;
//! use serde::Serialize;
//!
//! #[derive(Serialize, Clone, Debug)]
//! struct Event {
//!     id: u32,
//!     message: String,
//! }
//!
//! // Create Kafka configuration
//! let config = KafkaProducerConfig::default()
//!     .with_bootstrap_servers("localhost:9092")
//!     .with_topic("events");
//!
//! // Create a Kafka publish node
//! let kafka_publish = KafkaPublish::<Event>::new(config);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::KafkaPublish;
//! use streamweave::consumers::KafkaProducerConfig;
//! use streamweave::ErrorStrategy;
//! use serde::Serialize;
//!
//! # #[derive(Serialize, Clone, Debug)]
//! # struct Event { id: u32, message: String }
//! # let config = KafkaProducerConfig::default()
//! #     .with_bootstrap_servers("localhost:9092")
//! #     .with_topic("events");
//! // Create a Kafka publish node with error handling
//! let kafka_publish = KafkaPublish::<Event>::new(config)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("kafka-publisher".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Pass-Through Pattern**: Publishes data while passing it through for
//!   intermediate result capture
//! - **Kafka Integration**: Uses rdkafka or similar Kafka client for async
//!   Kafka publishing
//! - **Serializable Data**: Requires `Serialize` trait for flexible data
//!   structure support
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`KafkaPublish`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::consumers::KafkaProducerConfig;
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::KafkaPublishTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde::Serialize;
use std::pin::Pin;

/// Node that publishes data to Kafka while passing data through.
///
/// This node wraps `KafkaPublishTransformer` for use in graphs. It takes serializable data as input,
/// publishes it to a Kafka topic, and outputs the same data, enabling publishing to Kafka and continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{KafkaPublish, TransformerNode};
/// use crate::consumers::KafkaProducerConfig;
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
/// let kafka_publish = KafkaPublish::<Event>::new(config);
/// let node = TransformerNode::from_transformer(
///     "kafka_publish".to_string(),
///     kafka_publish,
/// );
/// ```
pub struct KafkaPublish<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying Kafka publish transformer
  transformer: KafkaPublishTransformer<T>,
}

impl<T> KafkaPublish<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `KafkaPublish` node with the given Kafka configuration.
  ///
  /// # Arguments
  ///
  /// * `kafka_config` - Kafka producer configuration.
  pub fn new(kafka_config: KafkaProducerConfig) -> Self {
    Self {
      transformer: KafkaPublishTransformer::new(kafka_config),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }

  /// Returns the Kafka producer configuration.
  #[must_use]
  pub fn kafka_config(&self) -> &KafkaProducerConfig {
    self.transformer.kafka_config()
  }
}

impl<T> Clone for KafkaPublish<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for KafkaPublish<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for KafkaPublish<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for KafkaPublish<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}

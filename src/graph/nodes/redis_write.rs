//! Redis write node for StreamWeave graphs
//!
//! Writes data to Redis Streams while passing data through. Takes serializable data as input,
//! writes to Redis, and outputs the same data, enabling writing to Redis and continuing processing.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::producers::RedisProducerConfig;
use crate::transformers::RedisWriteTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde::Serialize;
use std::pin::Pin;

/// Node that writes data to Redis Streams while passing data through.
///
/// This node wraps `RedisWriteTransformer` for use in graphs. It takes serializable data as input,
/// writes it to a Redis stream, and outputs the same data, enabling writing to Redis and continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{RedisWrite, TransformerNode};
/// use crate::producers::RedisProducerConfig;
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
/// let redis_write = RedisWrite::<Event>::new(config);
/// let node = TransformerNode::from_transformer(
///     "redis_write".to_string(),
///     redis_write,
/// );
/// ```
pub struct RedisWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying Redis write transformer
  transformer: RedisWriteTransformer<T>,
}

impl<T> RedisWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `RedisWrite` node with the given Redis configuration.
  ///
  /// # Arguments
  ///
  /// * `redis_config` - Redis Streams producer configuration.
  pub fn new(redis_config: RedisProducerConfig) -> Self {
    Self {
      transformer: RedisWriteTransformer::new(redis_config),
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

  /// Returns the Redis Streams producer configuration.
  #[must_use]
  pub fn redis_config(&self) -> &RedisProducerConfig {
    self.transformer.redis_config()
  }
}

impl<T> Clone for RedisWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for RedisWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for RedisWrite<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for RedisWrite<T>
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

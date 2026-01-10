use crate::error::{ComponentInfo, ErrorContext, ErrorStrategy};
use crate::producers::RedisProducerConfig;
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde::Serialize;
use std::pin::Pin;

/// A consumer that writes data to Redis Streams.
///
/// This consumer serializes items and writes them to a Redis stream using XADD.
/// It supports stream length limits and approximate maxlen for efficiency.
pub struct RedisConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Consumer configuration.
  pub config: ConsumerConfig<T>,
  /// Redis producer-specific configuration.
  pub redis_config: RedisProducerConfig,
}

impl<T> RedisConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new Redis consumer with the given configuration.
  #[must_use]
  pub fn new(redis_config: RedisProducerConfig) -> Self {
    Self {
      config: ConsumerConfig::default(),
      redis_config,
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

  /// Returns the Redis producer configuration.
  #[must_use]
  pub fn redis_config(&self) -> &RedisProducerConfig {
    &self.redis_config
  }
}

impl<T> Clone for RedisConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      redis_config: self.redis_config.clone(),
    }
  }
}

impl<T> Input for RedisConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

#[async_trait]
impl<T> Consumer for RedisConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) {
    // TODO: Implement Redis Streams writing
    // This is a placeholder implementation
    while let Some(_item) = stream.next().await {
      // Implementation needed: serialize and write to Redis using XADD
    }
  }

  fn get_config_impl(&self) -> &ConsumerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<T> {
    &mut self.config
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }
}

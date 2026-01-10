use crate::error::{ComponentInfo, ErrorAction, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A consumer that collects stream items into a Vec.
///
/// This consumer collects all items from the stream into a Vec.
/// Items are stored in the order they are received.
pub struct VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The Vec storing consumed items.
  pub vec: Vec<T>,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

impl<T> Default for VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `VecConsumer`.
  pub fn new() -> Self {
    Self {
      vec: Vec::new(),
      config: ConsumerConfig::default(),
    }
  }

  /// Creates a new `VecConsumer` with the specified capacity.
  ///
  /// # Arguments
  ///
  /// * `capacity` - The initial capacity of the Vec.
  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      vec: Vec::with_capacity(capacity),
      config: ConsumerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this consumer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this consumer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this consumer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  /// Consumes the consumer and returns the collected Vec.
  ///
  /// # Returns
  ///
  /// The Vec containing all consumed items.
  pub fn into_vec(self) -> Vec<T> {
    self.vec
  }
}

// Trait implementations for VecConsumer

impl<T> Input for VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

#[async_trait]
impl<T> Consumer for VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) {
    while let Some(item) = stream.next().await {
      self.vec.push(item);
    }
  }

  fn get_config_impl(&self) -> &ConsumerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<Self::Input> {
    &mut self.config
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<Self::Input>) {
    self.config = config;
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: "VecConsumer".to_string(),
    }
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Retry(_) => ErrorAction::Stop,
      ErrorStrategy::Custom(f) => f(error),
    }
  }
}

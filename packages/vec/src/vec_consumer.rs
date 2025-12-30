use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use streamweave::{Consumer, ConsumerConfig, Input};
use streamweave_error::ErrorStrategy;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};

/// A consumer that collects items into a `Vec`.
///
/// This consumer collects all items from the stream into an internal `Vec`,
/// preserving the order in which they were received.
#[derive(Clone)]
pub struct VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The internal `Vec` where items are collected.
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
  /// Creates a new `VecConsumer` with an empty `Vec`.
  pub fn new() -> Self {
    Self {
      vec: Vec::new(),
      config: ConsumerConfig::default(),
    }
  }

  /// Creates a new `VecConsumer` with a pre-allocated `Vec` capacity.
  ///
  /// # Arguments
  ///
  /// * `capacity` - The initial capacity of the internal `Vec`.
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

  /// Consumes the consumer and returns the collected `Vec`.
  ///
  /// # Returns
  ///
  /// The `Vec` containing all collected items in order.
  pub fn into_vec(self) -> Vec<T> {
    self.vec
  }
}

impl<T> Input for VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Consumer for VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let consumer_name = self.config.name.clone();
    println!("📥 [{}] Starting to consume stream", consumer_name);
    let mut count = 0;
    while let Some(value) = stream.next().await {
      count += 1;
      println!(
        "   📦 [{}] Consuming item #{}: {:?}",
        consumer_name, count, value
      );
      self.vec.push(value);
    }
    println!("✅ [{}] Finished consuming {} items", consumer_name, count);
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

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
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

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

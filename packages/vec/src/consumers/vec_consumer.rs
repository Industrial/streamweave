use streamweave::ConsumerConfig;
use streamweave_error::ErrorStrategy;

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

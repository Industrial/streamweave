use streamweave::ConsumerConfig;
use streamweave_error::ErrorStrategy;

/// A consumer that collects stream items into a fixed-size array.
///
/// This consumer collects up to `N` items from the stream into a fixed-size array.
/// Items are stored in the order they are received.
pub struct ArrayConsumer<T, const N: usize>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The array storing consumed items.
  pub array: [Option<T>; N],
  /// The current index for inserting items.
  pub index: usize,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

impl<T, const N: usize> Default for ArrayConsumer<T, N>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T, const N: usize> ArrayConsumer<T, N>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `ArrayConsumer`.
  pub fn new() -> Self {
    Self {
      array: std::array::from_fn(|_| None),
      index: 0,
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

  /// Consumes the consumer and returns the collected array.
  ///
  /// # Returns
  ///
  /// The array containing all consumed items.
  pub fn into_array(self) -> [Option<T>; N] {
    self.array
  }
}

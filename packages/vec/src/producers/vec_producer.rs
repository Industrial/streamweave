use streamweave::ProducerConfig;
use streamweave_error::ErrorStrategy;

/// A producer that yields items from a Vec.
///
/// This producer iterates over all items in the Vec and produces them
/// in order.
pub struct VecProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The Vec data to produce from.
  pub data: Vec<T>,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> VecProducer<T> {
  /// Creates a new `VecProducer` with the given Vec.
  ///
  /// # Arguments
  ///
  /// * `data` - The Vec to produce items from.
  pub fn new(data: Vec<T>) -> Self {
    Self {
      data,
      config: streamweave::ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

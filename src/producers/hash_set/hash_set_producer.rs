use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use std::collections::HashSet;
use std::hash::Hash;

/// A producer that generates items from a HashSet.
///
/// This producer iterates over the items in a HashSet and emits them as a stream.
pub struct HashSetProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  /// The HashSet containing the data to produce.
  pub data: HashSet<T>,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<T>,
}

impl<T> HashSetProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  /// Creates a new `HashSetProducer` with the given HashSet.
  ///
  /// # Arguments
  ///
  /// * `data` - The HashSet containing the items to produce.
  pub fn new(data: HashSet<T>) -> Self {
    Self {
      data,
      config: ProducerConfig::default(),
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

use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use std::collections::HashMap;
use std::hash::Hash;

/// A producer that yields key-value pairs from a HashMap.
///
/// This producer iterates over all entries in the HashMap and produces
/// them as `(K, V)` tuples.
pub struct HashMapProducer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The HashMap data to produce from.
  pub data: HashMap<K, V>,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<(K, V)>,
}

impl<K, V> HashMapProducer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `HashMapProducer` with the given HashMap.
  ///
  /// # Arguments
  ///
  /// * `data` - The HashMap to produce items from.
  pub fn new(data: HashMap<K, V>) -> Self {
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
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<(K, V)>) -> Self {
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

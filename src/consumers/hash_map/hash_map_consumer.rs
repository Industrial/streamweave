use crate::consumer::ConsumerConfig;
use crate::error::ErrorStrategy;
use std::collections::HashMap;
use std::hash::Hash;

/// A consumer that collects key-value pairs into a `HashMap`.
///
/// This consumer expects items of type `(K, V)` and collects them into an internal `HashMap`.
/// If duplicate keys are encountered, the last value will overwrite previous ones.
pub struct HashMapConsumer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The internal `HashMap` where key-value pairs are collected.
  pub map: HashMap<K, V>,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<(K, V)>,
}

impl<K, V> Default for HashMapConsumer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<K, V> HashMapConsumer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `HashMapConsumer` with an empty `HashMap`.
  pub fn new() -> Self {
    Self {
      map: HashMap::new(),
      config: ConsumerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this consumer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<(K, V)>) -> Self {
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

  /// Consumes the consumer and returns the collected `HashMap`.
  ///
  /// # Returns
  ///
  /// The `HashMap` containing all collected key-value pairs.
  pub fn into_map(self) -> HashMap<K, V> {
    self.map
  }
}

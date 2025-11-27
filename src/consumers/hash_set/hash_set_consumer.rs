use crate::consumer::ConsumerConfig;
use crate::error::ErrorStrategy;
use std::collections::HashSet;
use std::hash::Hash;

/// A consumer that collects items into a `HashSet`.
///
/// This consumer collects all items from the stream into an internal `HashSet`,
/// automatically deduplicating any duplicate items.
pub struct HashSetConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  /// The internal `HashSet` where items are collected.
  pub set: HashSet<T>,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

impl<T> Default for HashSetConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> HashSetConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  /// Creates a new `HashSetConsumer` with an empty `HashSet`.
  pub fn new() -> Self {
    Self {
      set: HashSet::new(),
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

  /// Consumes the consumer and returns the collected `HashSet`.
  ///
  /// # Returns
  ///
  /// The `HashSet` containing all collected items (without duplicates).
  pub fn into_set(self) -> HashSet<T> {
    self.set
  }
}

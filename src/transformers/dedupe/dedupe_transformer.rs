use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::collections::HashSet;
use std::hash::Hash;

/// A transformer that removes duplicate items from a stream.
///
/// This transformer tracks all items that have been seen and only passes through
/// items that have not been encountered before, effectively deduplicating the stream.
pub struct DedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  /// A set of items that have already been seen.
  pub seen: HashSet<T>,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
}

impl<T> Default for DedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> DedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  /// Creates a new `DedupeTransformer`.
  pub fn new() -> Self {
    Self {
      seen: HashSet::new(),
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::hash::Hash;
use std::marker::PhantomData;

/// A transformer that groups stream items by a key function.
///
/// This transformer groups items from the input stream based on a key extracted
/// by the provided function, producing groups of items with the same key.
#[derive(Clone)]
pub struct GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  /// The function used to extract the grouping key from each item.
  pub key_fn: F,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the item type parameter.
  pub _phantom_t: PhantomData<T>,
  /// Phantom data to track the key type parameter.
  pub _phantom_k: PhantomData<K>,
}

impl<F, T, K> GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  /// Creates a new `GroupByTransformer` with the given key function.
  ///
  /// # Arguments
  ///
  /// * `key_fn` - The function to extract the grouping key from each item.
  pub fn new(key_fn: F) -> Self {
    Self {
      key_fn,
      config: TransformerConfig::default(),
      _phantom_t: PhantomData,
      _phantom_k: PhantomData,
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }
}

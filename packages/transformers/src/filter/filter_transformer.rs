use streamweave::TransformerConfig;
use streamweave_error::ErrorStrategy;

/// A transformer that filters stream items based on a predicate function.
///
/// Only items for which the predicate returns `true` are passed through.
pub struct FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The predicate function used to filter items.
  pub predicate: F,
  /// Phantom data to track the item type.
  pub _phantom: std::marker::PhantomData<T>,
  /// Configuration for the transformer.
  pub config: TransformerConfig<T>,
}

impl<F, T> FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `FilterTransformer` with the given predicate function.
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
      _phantom: std::marker::PhantomData,
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

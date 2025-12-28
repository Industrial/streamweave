use std::marker::PhantomData;
use streamweave::TransformerConfig;
use streamweave_error::ErrorStrategy;

/// A transformer that splits a stream of items into groups based on a predicate.
///
/// This transformer groups consecutive items where the predicate returns `true`
/// into separate vectors, effectively splitting the stream at points where the
/// predicate returns `false`.
#[derive(Clone)]
pub struct SplitTransformer<F, T>
where
  F: Send + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The predicate function used to determine split points.
  pub predicate: F,
  /// Phantom data to track the item type parameter.
  pub _phantom: std::marker::PhantomData<T>,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<Vec<T>>,
}

impl<F, T> SplitTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `SplitTransformer` with the given predicate.
  ///
  /// # Arguments
  ///
  /// * `predicate` - The function to use for determining split points.
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
      _phantom: PhantomData,
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Vec<T>>) -> Self {
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

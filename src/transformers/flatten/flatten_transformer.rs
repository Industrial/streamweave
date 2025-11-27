use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

/// A transformer that flattens a stream of vectors into a stream of individual items.
///
/// This transformer takes a stream of `Vec<T>` and produces a stream of `T` by
/// flattening all vectors into a single stream.
#[derive(Clone)]
pub struct FlattenTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Phantom data to track the item type.
  pub _phantom: PhantomData<T>,
  /// Configuration for the transformer.
  pub config: TransformerConfig<Vec<T>>,
}

impl<T> Default for FlattenTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> FlattenTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `FlattenTransformer`.
  pub fn new() -> Self {
    Self {
      _phantom: PhantomData,
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Vec<T>>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }
}

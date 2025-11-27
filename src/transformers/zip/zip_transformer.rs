use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

/// A transformer that zips items from multiple streams into vectors.
///
/// This transformer collects items from multiple input streams and combines
/// them into vectors, emitting one vector per combination of items from each stream.
#[derive(Clone)]
pub struct ZipTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<Vec<T>>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Default for ZipTransformer<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> ZipTransformer<T> {
  /// Creates a new `ZipTransformer`.
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::<Vec<T>>::default(),
      _phantom: PhantomData,
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

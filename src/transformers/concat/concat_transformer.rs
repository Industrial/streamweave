use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use futures::Stream;
use std::pin::Pin;

/// A transformer that concatenates items from another stream to the input stream.
///
/// This transformer appends all items from the `other` stream after all items
/// from the input stream have been processed, creating a concatenated output stream.
pub struct ConcatTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The other stream to concatenate with the input stream.
  pub other: Pin<Box<dyn Stream<Item = T> + Send>>,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: std::marker::PhantomData<T>,
}

impl<T> ConcatTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `ConcatTransformer` with the given other stream.
  ///
  /// # Arguments
  ///
  /// * `other` - The stream to concatenate with the input stream.
  pub fn new(other: Pin<Box<dyn Stream<Item = T> + Send>>) -> Self {
    Self {
      other,
      config: TransformerConfig::default(),
      _phantom: std::marker::PhantomData,
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

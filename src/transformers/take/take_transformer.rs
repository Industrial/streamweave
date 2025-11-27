use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

/// A transformer that takes a specified number of items from a stream.
///
/// This transformer passes through only the first `take` items from the input
/// stream and then stops producing items.
#[derive(Clone)]
pub struct TakeTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// The number of items to take from the stream.
  pub take: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: std::marker::PhantomData<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> TakeTransformer<T> {
  /// Creates a new `TakeTransformer` with the given take count.
  ///
  /// # Arguments
  ///
  /// * `take` - The number of items to take from the stream.
  pub fn new(take: usize) -> Self {
    Self {
      take,
      config: TransformerConfig::<T>::default(),
      _phantom: PhantomData,
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

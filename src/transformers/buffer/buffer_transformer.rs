use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

/// A transformer that buffers items in a stream up to a specified capacity.
///
/// This transformer maintains an internal buffer and only emits items when
/// the buffer reaches the specified `capacity`.
pub struct BufferTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The maximum capacity of the buffer.
  pub capacity: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the input type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> BufferTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `BufferTransformer` with the given capacity.
  ///
  /// # Arguments
  ///
  /// * `capacity` - The maximum capacity of the buffer.
  pub fn new(capacity: usize) -> Self {
    Self {
      capacity,
      config: TransformerConfig::default(),
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

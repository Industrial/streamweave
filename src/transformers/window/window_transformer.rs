use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;

/// A transformer that creates sliding windows of items from a stream.
///
/// This transformer groups consecutive items into windows of a specified size,
/// producing vectors of items as the window slides over the input stream.
#[derive(Clone)]
pub struct WindowTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// The size of each window.
  pub size: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: std::marker::PhantomData<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> WindowTransformer<T> {
  /// Creates a new `WindowTransformer` with the given window size.
  ///
  /// # Arguments
  ///
  /// * `size` - The size of each window.
  pub fn new(size: usize) -> Self {
    Self {
      size,
      config: TransformerConfig::<T>::default(),
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

use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;

/// A transformer that manages backpressure by buffering items.
///
/// This transformer helps control the flow of data through a pipeline by
/// maintaining a buffer of items, allowing downstream components to process
/// at their own pace.
pub struct BackpressureTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The maximum size of the buffer for managing backpressure.
  pub buffer_size: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
}

impl<T> BackpressureTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `BackpressureTransformer` with the given buffer size.
  ///
  /// # Arguments
  ///
  /// * `buffer_size` - The maximum size of the buffer.
  pub fn new(buffer_size: usize) -> Self {
    Self {
      buffer_size,
      config: TransformerConfig::default(),
    }
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

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  /// Sets the buffer size for this transformer.
  ///
  /// # Arguments
  ///
  /// * `buffer_size` - The maximum size of the buffer.
  pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
    self.buffer_size = buffer_size;
    self
  }
}

impl<T> Clone for BackpressureTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      buffer_size: self.buffer_size,
      config: self.config.clone(),
    }
  }
}

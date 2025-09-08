use crate::error::ErrorStrategy;
use crate::structs::transformers::backpressure::BackpressureTransformer;
use crate::traits::transformer::TransformerConfig;

impl<T> BackpressureTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(buffer_size: usize) -> Self {
    Self {
      buffer_size,
      config: TransformerConfig::default(),
    }
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

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

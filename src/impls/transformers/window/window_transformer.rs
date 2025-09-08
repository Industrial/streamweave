use crate::error::ErrorStrategy;
use crate::structs::transformers::window::WindowTransformer;
use crate::traits::transformer::TransformerConfig;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> WindowTransformer<T> {
  pub fn new(size: usize) -> Self {
    Self {
      size,
      config: TransformerConfig::<T>::default(),
      _phantom: std::marker::PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

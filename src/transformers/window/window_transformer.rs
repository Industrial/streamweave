use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;

#[derive(Clone)]
pub struct WindowTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  pub size: usize,
  pub config: TransformerConfig<T>,
  pub _phantom: std::marker::PhantomData<T>,
}

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

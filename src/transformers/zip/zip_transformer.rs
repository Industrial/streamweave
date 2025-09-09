use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct ZipTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  pub config: TransformerConfig<Vec<T>>,
  pub _phantom: PhantomData<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Default for ZipTransformer<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> ZipTransformer<T> {
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::<Vec<T>>::default(),
      _phantom: PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Vec<T>>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

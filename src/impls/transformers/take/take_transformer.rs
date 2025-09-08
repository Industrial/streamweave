use crate::error::ErrorStrategy;
use crate::structs::transformers::take::TakeTransformer;
use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> TakeTransformer<T> {
  pub fn new(take: usize) -> Self {
    Self {
      take,
      config: TransformerConfig::<T>::default(),
      _phantom: PhantomData,
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

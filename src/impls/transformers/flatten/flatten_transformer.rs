use crate::error::ErrorStrategy;
use crate::structs::transformers::flatten::FlattenTransformer;
use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

impl<T> FlattenTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new() -> Self {
    Self {
      _phantom: PhantomData,
      config: TransformerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Vec<T>>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }
}

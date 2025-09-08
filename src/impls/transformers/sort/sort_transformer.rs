use crate::error::ErrorStrategy;
use crate::structs::transformers::sort::SortTransformer;
use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

impl<T> SortTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
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

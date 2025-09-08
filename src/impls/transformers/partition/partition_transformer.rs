use crate::error::ErrorStrategy;
use crate::structs::transformers::partition::PartitionTransformer;
use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

impl<F, T> PartitionTransformer<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }
}

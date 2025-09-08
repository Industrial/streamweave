use crate::error::ErrorStrategy;
use crate::structs::transformers::split::SplitTransformer;
use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

impl<F, T> SplitTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
      _phantom: PhantomData,
      config: TransformerConfig::default(),
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

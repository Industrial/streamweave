use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::hash::Hash;
use std::marker::PhantomData;

pub struct DistinctTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}

impl<T> Default for DistinctTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> DistinctTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  pub fn new() -> Self {
    Self {
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

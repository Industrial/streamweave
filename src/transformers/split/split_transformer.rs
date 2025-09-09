use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct SplitTransformer<F, T>
where
  F: Send + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub predicate: F,
  pub _phantom: std::marker::PhantomData<T>,
  pub config: TransformerConfig<Vec<T>>,
}

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

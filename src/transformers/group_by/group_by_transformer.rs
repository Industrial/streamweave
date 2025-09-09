use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::hash::Hash;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  pub key_fn: F,
  pub config: TransformerConfig<T>,
  pub _phantom_t: PhantomData<T>,
  pub _phantom_k: PhantomData<K>,
}

impl<F, T, K> GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  pub fn new(key_fn: F) -> Self {
    Self {
      key_fn,
      config: TransformerConfig::default(),
      _phantom_t: PhantomData,
      _phantom_k: PhantomData,
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

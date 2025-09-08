use crate::error::ErrorStrategy;
use crate::structs::transformers::group_by::GroupByTransformer;
use crate::traits::transformer::TransformerConfig;
use std::hash::Hash;
use std::marker::PhantomData;

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

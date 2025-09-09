use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::collections::HashSet;
use std::hash::Hash;

pub struct DedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  pub seen: HashSet<T>,
  pub config: TransformerConfig<T>,
}

impl<T> Default for DedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> DedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  pub fn new() -> Self {
    Self {
      seen: HashSet::new(),
      config: TransformerConfig::default(),
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

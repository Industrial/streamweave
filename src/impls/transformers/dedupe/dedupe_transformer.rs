use crate::error::ErrorStrategy;
use crate::structs::transformers::dedupe::DedupeTransformer;
use crate::traits::transformer::TransformerConfig;
use std::collections::HashSet;
use std::hash::Hash;

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

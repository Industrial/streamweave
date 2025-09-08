use crate::error::ErrorStrategy;
use crate::structs::consumers::hash_set::HashSetConsumer;
use crate::traits::consumer::ConsumerConfig;
use std::collections::HashSet;
use std::hash::Hash;

impl<T> HashSetConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  pub fn new() -> Self {
    Self {
      set: HashSet::new(),
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  pub fn into_set(self) -> HashSet<T> {
    self.set
  }
}

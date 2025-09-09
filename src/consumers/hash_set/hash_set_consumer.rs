use crate::error::ErrorStrategy;
use crate::consumer::ConsumerConfig;
use std::collections::HashSet;
use std::hash::Hash;

pub struct HashSetConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  pub set: HashSet<T>,
  pub config: ConsumerConfig<T>,
}

impl<T> Default for HashSetConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

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

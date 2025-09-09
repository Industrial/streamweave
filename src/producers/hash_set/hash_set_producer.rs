use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use std::collections::HashSet;
use std::hash::Hash;

pub struct HashSetProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  pub data: HashSet<T>,
  pub config: ProducerConfig<T>,
}

impl<T> HashSetProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  pub fn new(data: HashSet<T>) -> Self {
    Self {
      data,
      config: ProducerConfig::default(),
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

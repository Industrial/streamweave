use crate::error::ErrorStrategy;
use crate::structs::producers::hash_set::HashSetProducer;
use crate::traits::producer::ProducerConfig;
use std::collections::HashSet;
use std::hash::Hash;

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

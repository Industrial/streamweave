use crate::error::ErrorStrategy;
use crate::structs::consumers::hash_map::HashMapConsumer;
use crate::traits::consumer::ConsumerConfig;
use std::collections::HashMap;
use std::hash::Hash;

impl<K, V> Default for HashMapConsumer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<K, V> HashMapConsumer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new() -> Self {
    Self {
      map: HashMap::new(),
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<(K, V)>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  pub fn into_map(self) -> HashMap<K, V> {
    self.map
  }
}

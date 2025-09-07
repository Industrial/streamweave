use crate::error::ErrorStrategy;
use crate::structs::hash_map_producer::HashMapProducer;
use crate::traits::producer::ProducerConfig;
use std::collections::HashMap;
use std::hash::Hash;

impl<K, V> HashMapProducer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(data: HashMap<K, V>) -> Self {
    Self {
      data,
      config: ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<(K, V)>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

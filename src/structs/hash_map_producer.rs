use crate::traits::producer::ProducerConfig;
use std::collections::HashMap;
use std::hash::Hash;

pub struct HashMapProducer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub data: HashMap<K, V>,
  pub config: ProducerConfig<(K, V)>,
}

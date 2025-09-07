use crate::traits::consumer::ConsumerConfig;
use std::collections::HashMap;
use std::hash::Hash;

pub struct HashMapConsumer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub map: HashMap<K, V>,
  pub config: ConsumerConfig<(K, V)>,
}

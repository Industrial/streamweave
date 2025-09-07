use crate::traits::producer::ProducerConfig;
use std::collections::HashSet;
use std::hash::Hash;

pub struct HashSetProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  pub data: HashSet<T>,
  pub config: ProducerConfig<T>,
}

use crate::traits::consumer::ConsumerConfig;
use std::collections::HashSet;
use std::hash::Hash;

pub struct HashSetConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  pub set: HashSet<T>,
  pub config: ConsumerConfig<T>,
}

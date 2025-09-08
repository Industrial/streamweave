use crate::structs::producers::hash_map::HashMapProducer;
use crate::traits::output::Output;
use futures::Stream;
use std::hash::Hash;
use std::pin::Pin;

impl<K, V> Output for HashMapProducer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = (K, V);
  type OutputStream = Pin<Box<dyn Stream<Item = (K, V)> + Send>>;
}

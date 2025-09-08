use crate::structs::producers::hash_set::HashSetProducer;
use crate::traits::output::Output;
use futures::Stream;
use std::hash::Hash;
use std::pin::Pin;

impl<T> Output for HashSetProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

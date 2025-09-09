use crate::output::Output;
use crate::transformers::dedupe::dedupe_transformer::DedupeTransformer;
use futures::Stream;
use std::hash::Hash;
use std::pin::Pin;

impl<T> Output for DedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

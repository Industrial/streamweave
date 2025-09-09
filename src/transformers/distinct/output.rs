use crate::transformers::distinct::distinct_transformer::DistinctTransformer;
use crate::output::Output;
use futures::Stream;
use std::hash::Hash;
use std::pin::Pin;

impl<T> Output for DistinctTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

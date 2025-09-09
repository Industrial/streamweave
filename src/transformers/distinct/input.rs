use crate::input::Input;
use crate::transformers::distinct::distinct_transformer::DistinctTransformer;
use futures::Stream;
use std::hash::Hash;
use std::pin::Pin;

impl<T> Input for DistinctTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

use crate::structs::transformers::distinct::DistinctTransformer;
use crate::traits::input::Input;
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

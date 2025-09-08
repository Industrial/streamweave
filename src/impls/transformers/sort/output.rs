use crate::structs::transformers::sort::SortTransformer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for SortTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

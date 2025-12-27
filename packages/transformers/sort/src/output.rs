use crate::sort_transformer::SortTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl<T> Output for SortTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

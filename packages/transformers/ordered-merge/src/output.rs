use crate::ordered_merge_transformer::OrderedMergeTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Output;

impl<T> Output for OrderedMergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

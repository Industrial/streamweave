use crate::output::Output;
use crate::transformers::ordered_merge::ordered_merge_transformer::OrderedMergeTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for OrderedMergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

use crate::input::Input;
use crate::transformers::ordered_merge::ordered_merge_transformer::OrderedMergeTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for OrderedMergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

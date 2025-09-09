use crate::transformers::merge::merge_transformer::MergeTransformer;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

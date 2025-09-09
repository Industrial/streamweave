use crate::input::Input;
use crate::transformers::sort::sort_transformer::SortTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for SortTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

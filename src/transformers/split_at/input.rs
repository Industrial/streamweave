use crate::transformers::split_at::split_at_transformer::SplitAtTransformer;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for SplitAtTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

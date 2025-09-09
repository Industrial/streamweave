use crate::input::Input;
use crate::transformers::interleave::interleave_transformer::InterleaveTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for InterleaveTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

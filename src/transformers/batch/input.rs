use crate::transformers::batch::batch_transformer::BatchTransformer;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

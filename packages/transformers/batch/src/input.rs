use crate::batch_transformer::BatchTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Input;

impl<T> Input for BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

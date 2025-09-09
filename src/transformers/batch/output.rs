use crate::transformers::batch::batch_transformer::BatchTransformer;
use crate::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

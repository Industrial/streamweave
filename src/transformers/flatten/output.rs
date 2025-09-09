use crate::output::Output;
use crate::transformers::flatten::flatten_transformer::FlattenTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for FlattenTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

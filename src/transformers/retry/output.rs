use crate::output::Output;
use crate::transformers::retry::retry_transformer::RetryTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for RetryTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

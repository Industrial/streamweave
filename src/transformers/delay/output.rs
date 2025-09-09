use crate::output::Output;
use crate::transformers::delay::delay_transformer::DelayTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for DelayTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

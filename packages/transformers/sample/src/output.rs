use crate::sample_transformer::SampleTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Output;

impl<T> Output for SampleTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

use super::sample_transformer::SampleTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl<T> Output for SampleTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

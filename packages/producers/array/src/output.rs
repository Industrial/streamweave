use super::array_producer::ArrayProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl<T: Send + Sync + 'static + Clone + std::fmt::Debug, const N: usize> Output
  for ArrayProducer<T, N>
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

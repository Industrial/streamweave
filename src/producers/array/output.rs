use super::array_producer::ArrayProducer;
use crate::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<T: Send + Sync + 'static + Clone + std::fmt::Debug, const N: usize> Output
  for ArrayProducer<T, N>
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

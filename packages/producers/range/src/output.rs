use super::range_producer::RangeProducer;
use futures::Stream;
use num_traits::Num;
use std::pin::Pin;
use streamweave::Output;

impl<T> Output for RangeProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Num + Copy + PartialOrd + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

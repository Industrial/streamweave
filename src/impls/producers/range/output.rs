use crate::structs::producers::range::RangeProducer;
use crate::traits::output::Output;
use futures::Stream;
use num_traits::Num;
use std::pin::Pin;

impl<T> Output for RangeProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Num + Copy + PartialOrd + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

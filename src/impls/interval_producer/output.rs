use crate::structs::interval_producer::IntervalProducer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl Output for IntervalProducer {
  type Output = ();
  type OutputStream = Pin<Box<dyn Stream<Item = ()> + Send>>;
}

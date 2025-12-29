use super::interval_producer::IntervalProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for IntervalProducer {
  type Output = std::time::SystemTime;
  type OutputStream = Pin<Box<dyn Stream<Item = std::time::SystemTime> + Send>>;
}

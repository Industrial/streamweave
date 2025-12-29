use super::signal_producer::{Signal, SignalProducer};
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for SignalProducer {
  type Output = Signal;
  type OutputStream = Pin<Box<dyn Stream<Item = Signal> + Send>>;
}

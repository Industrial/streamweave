use super::stdin_producer::StdinProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for StdinProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

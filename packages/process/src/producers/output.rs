use super::process_producer::ProcessProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for ProcessProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

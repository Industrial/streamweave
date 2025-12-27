use super::command_producer::CommandProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for CommandProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

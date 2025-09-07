use crate::structs::command_producer::CommandProducer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl Output for CommandProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

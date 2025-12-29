use super::directory_producer::DirectoryProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for DirectoryProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

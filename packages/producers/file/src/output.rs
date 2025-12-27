use super::file_producer::FileProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for FileProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

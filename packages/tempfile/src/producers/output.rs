use super::tempfile_producer::TempFileProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for TempFileProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

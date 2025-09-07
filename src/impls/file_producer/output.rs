use crate::structs::file_producer::FileProducer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl Output for FileProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

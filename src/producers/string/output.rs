use super::string_producer::StringProducer;
use crate::output::Output;
use futures::Stream;
use std::pin::Pin;

impl Output for StringProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

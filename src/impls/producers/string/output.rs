use crate::structs::producers::string::StringProducer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl Output for StringProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

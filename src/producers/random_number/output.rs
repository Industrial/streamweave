use super::random_number_producer::RandomNumberProducer;
use crate::output::Output;
use futures::Stream;
use std::pin::Pin;

impl Output for RandomNumberProducer {
  type Output = i32;
  type OutputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
}

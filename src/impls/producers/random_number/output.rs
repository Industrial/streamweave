use crate::structs::producers::random_number::RandomNumberProducer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl Output for RandomNumberProducer {
  type Output = i32;
  type OutputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
}
